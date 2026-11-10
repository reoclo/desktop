use crate::{
    control::Client,
    ui::{Fault, Request, State, ViewState},
};
use rune::AppHandle;
use std::sync::mpsc::{self, Sender};
use std::time::Duration;

pub struct Connection {
    stop: Sender<()>,
}

fn deliver(
    handles: &[AppHandle<State>],
    request: &Request,
    result: Result<serde_json::Value, Fault>,
) {
    for handle in handles {
        let request = request.clone();
        let result = result.clone();
        handle.update(move |state, _| state.receive(&request, result));
    }
}

fn refresh(client: &Client, handles: &[AppHandle<State>]) {
    for path in [
        "/v1/status",
        "/v1/tunnels",
        "/v1/foreground",
        "/v1/orgs",
        "/v1/approvals",
        "/v1/settings",
        "/v1/hosts",
    ] {
        let request = Request {
            method: "GET".into(),
            path: path.into(),
            body: None,
        };
        deliver(
            handles,
            &request,
            client.request("GET", path, None).map_err(Fault::from),
        );
    }
}

pub fn backoff(attempt: u32) -> Duration {
    Duration::from_secs(1_u64 << attempt.min(5))
}

impl Connection {
    pub fn start(client: Client, handles: Vec<AppHandle<State>>) -> (Self, Sender<Request>) {
        let (send, receive) = mpsc::channel::<Request>();
        let (stop, stopped) = mpsc::channel();
        let requests = client.clone();
        let updates = handles.clone();
        std::thread::spawn(move || {
            refresh(&requests, &updates);
            for request in receive {
                let response = requests
                    .request(&request.method, &request.path, request.body.as_ref())
                    .map_err(Fault::from);
                let succeeded = response.is_ok();
                deliver(&updates, &request, response);
                if succeeded && request.method != "GET" {
                    refresh(&requests, &updates);
                }
            }
        });
        std::thread::spawn(move || {
            let mut attempt: u32 = 0;
            loop {
                if stopped.try_recv().is_ok() {
                    break;
                }
                match client.events() {
                    Ok(mut events) => {
                        attempt = 0;
                        refresh(&client, &handles);
                        loop {
                            if stopped.try_recv().is_ok() {
                                return;
                            }
                            match events.next_event() {
                                Ok(Some(event)) => {
                                    for handle in &handles {
                                        let name = event.name.clone();
                                        let data = event.data.clone();
                                        handle.update(move |s, _| s.apply_event(&name, &data));
                                    }
                                    if event.name == "auth" {
                                        let auth: Result<crate::model::AuthEvent, _> =
                                            serde_json::from_str(&event.data);
                                        if auth.is_ok_and(|auth| auth.status == "success") {
                                            refresh(&client, &handles);
                                        }
                                    }
                                }
                                Ok(None) | Err(_) => break,
                            }
                        }
                    }
                    Err(error) => {
                        for handle in &handles {
                            let message = error.to_string();
                            handle.update(move |s, _| {
                                s.phase = ViewState::Offline;
                                s.error = Some(message);
                            });
                        }
                    }
                }
                match stopped.recv_timeout(backoff(attempt)) {
                    Ok(()) | Err(mpsc::RecvTimeoutError::Disconnected) => break,
                    Err(mpsc::RecvTimeoutError::Timeout) => attempt = attempt.saturating_add(1),
                }
            }
        });
        (Self { stop }, send)
    }
}

impl Drop for Connection {
    fn drop(&mut self) {
        if let Err(error) = self.stop.send(()) {
            eprintln!("Event worker already stopped: {error}");
        }
    }
}
