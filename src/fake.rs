mod routes;
use crate::control::Error;
use crate::model::*;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::io::{BufRead, BufReader, Read, Write};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, mpsc};
use std::thread::JoinHandle;
use std::time::Duration;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Fixture {
    pub status: Status,
    pub host_entries: Vec<HostEntry>,
    pub tunnels: Vec<TunnelView>,
    pub approvals: Vec<Approval>,
    #[serde(default)]
    pub approval_history: Vec<Approval>,
    pub orgs: Vec<Org>,
    pub servers: Vec<Server>,
    pub ports: Vec<Port>,
    #[serde(default)]
    pub settings: Settings,
}

impl Fixture {
    pub fn load() -> Result<Self, serde_json::Error> {
        serde_json::from_str(include_str!("../fixtures/daemon.json"))
    }
}

struct Shared {
    fixture: Fixture,
    subscribers: Vec<mpsc::Sender<String>>,
    stopped: bool,
}

pub struct FakeDaemon {
    path: PathBuf,
    shared: Arc<Mutex<Shared>>,
    subscribed: mpsc::Receiver<()>,
    thread: Option<JoinHandle<()>>,
}

impl FakeDaemon {
    pub fn start(path: PathBuf) -> Result<Self, Error> {
        let listener = UnixListener::bind(&path)?;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))?;
        let shared = Arc::new(Mutex::new(Shared {
            fixture: Fixture::load()?,
            subscribers: Vec::new(),
            stopped: false,
        }));
        let (ready, subscribed) = mpsc::channel();
        let state = Arc::clone(&shared);
        let thread = std::thread::spawn(move || {
            for connection in listener.incoming() {
                let Ok(stream) = connection else { break };
                if state.lock().map_or(true, |s| s.stopped) {
                    break;
                }
                let state = Arc::clone(&state);
                let ready = ready.clone();
                std::thread::spawn(move || {
                    if let Err(error) = serve(stream, state, ready) {
                        eprintln!("fake daemon connection ended: {error}");
                    }
                });
            }
        });
        Ok(Self {
            path,
            shared,
            subscribed,
            thread: Some(thread),
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn wait_for_subscriber(&self, timeout: Duration) -> Result<(), mpsc::RecvTimeoutError> {
        self.subscribed.recv_timeout(timeout)
    }
}

impl Drop for FakeDaemon {
    fn drop(&mut self) {
        if let Ok(mut state) = self.shared.lock() {
            state.stopped = true;
            state.subscribers.clear();
        }
        if let Err(error) = UnixStream::connect(&self.path) {
            eprintln!("fake daemon shutdown wake: {error}");
        }
        if let Some(thread) = self.thread.take()
            && thread.join().is_err()
        {
            eprintln!("fake daemon listener panicked");
        }
        if let Err(error) = std::fs::remove_file(&self.path) {
            eprintln!("fake daemon socket cleanup: {error}");
        }
    }
}

fn serve(
    mut stream: UnixStream,
    shared: Arc<Mutex<Shared>>,
    ready: mpsc::Sender<()>,
) -> Result<(), Error> {
    stream.set_read_timeout(Some(Duration::from_secs(5)))?;
    stream.set_write_timeout(Some(Duration::from_secs(5)))?;
    let mut reader = BufReader::new(stream.try_clone()?);
    let mut line = String::new();
    reader.read_line(&mut line)?;
    let words: Vec<_> = line.split_whitespace().collect();
    let method = words.first().copied().unwrap_or("");
    let path = words.get(1).copied().unwrap_or("");
    let mut len = 0;
    let mut app = false;
    loop {
        let mut header = String::new();
        reader.read_line(&mut header)?;
        if header == "\r\n" || header.is_empty() {
            break;
        }
        if let Some((key, value)) = header.split_once(':') {
            if key.eq_ignore_ascii_case("content-length") {
                len = value.trim().parse().unwrap_or(0);
            }
            if key.eq_ignore_ascii_case("x-reoclo-client") {
                app = value.trim() == "app";
            }
        }
    }
    if len > 1024 * 1024 {
        return Err(Error::Protocol("Fake request too large".into()));
    }
    let mut payload = vec![0; len];
    reader.read_exact(&mut payload)?;
    let input = if payload.is_empty() {
        json!({})
    } else {
        serde_json::from_slice(&payload)?
    };
    if !app {
        return response(
            &mut stream,
            400,
            &json!({"error":{"code":"invalid","message":"Missing app client header"}}),
        );
    }
    if path == "/v1/events" {
        let (send, receive) = mpsc::channel();
        shared
            .lock()
            .map_err(|_| Error::Protocol("Fake state poisoned".into()))?
            .subscribers
            .push(send);
        stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nTransfer-Encoding: chunked\r\n\r\n")?;
        chunk(&mut stream, ": connected\n\n")?;
        if let Err(error) = ready.send(()) {
            eprintln!("fake subscriber signal: {error}");
        }
        loop {
            match receive.recv_timeout(Duration::from_secs(5)) {
                Ok(event) => chunk(&mut stream, &event)?,
                Err(mpsc::RecvTimeoutError::Timeout) => chunk(&mut stream, ": ping\n\n")?,
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
            }
        }
        return Ok(());
    }
    let mut state = shared
        .lock()
        .map_err(|_| Error::Protocol("Fake state poisoned".into()))?;
    let (status, body, event) = routes::route(&mut state.fixture, method, path, input);
    if let Some(event) = event {
        state
            .subscribers
            .retain(|send| send.send(event.clone()).is_ok());
    }
    response(&mut stream, status, &body)
}

fn chunk(stream: &mut UnixStream, value: &str) -> Result<(), Error> {
    write!(stream, "{:x}\r\n{value}\r\n", value.len())?;
    Ok(())
}

fn response(stream: &mut UnixStream, status: u16, body: &Value) -> Result<(), Error> {
    let bytes = serde_json::to_vec(body)?;
    write!(
        stream,
        "HTTP/1.1 {status} Response\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        bytes.len()
    )?;
    stream.write_all(&bytes)?;
    Ok(())
}
