mod body;
pub use body::Events;
use body::Response;
use serde::{Deserialize, de::DeserializeOwned};
use serde_json::Value;
use std::io::{BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::time::Duration;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("The local daemon could not be reached: {0}")]
    Io(#[from] std::io::Error),
    #[error("Invalid daemon response: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Invalid control protocol: {0}")]
    Protocol(String),
    #[error("{code}: {message} (HTTP {status})")]
    Api {
        status: u16,
        code: String,
        message: String,
        state: Option<String>,
    },
}

#[derive(Clone, Debug)]
pub struct Client {
    path: PathBuf,
}

impl Client {
    pub fn socket_path(&self) -> &Path {
        &self.path
    }

    pub fn new(path: impl AsRef<Path>) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
        }
    }

    fn connect(&self, method: &str, path: &str, body: Option<&Value>) -> Result<Response, Error> {
        if !path.starts_with("/v1/")
            || path.contains(['\r', '\n', ' '])
            || !matches!(method, "GET" | "POST" | "PATCH" | "DELETE")
        {
            return Err(Error::Protocol("Invalid request path or method".into()));
        }
        let mut stream = UnixStream::connect(&self.path)?;
        // Trusting the CA waits on the person at the macOS password prompt, and a port scan
        // runs on the server for up to 45 seconds.
        let wait = if path.starts_with("/v1/ca/") {
            300
        } else if path.ends_with("/ports") {
            60
        } else {
            15
        };
        stream.set_read_timeout(Some(Duration::from_secs(wait)))?;
        stream.set_write_timeout(Some(Duration::from_secs(5)))?;
        let payload = body
            .map(serde_json::to_vec)
            .transpose()?
            .unwrap_or_default();
        write!(
            stream,
            "{method} {path} HTTP/1.1\r\nHost: localhost\r\nX-Reoclo-Client: app\r\nAccept: application/json, text/event-stream\r\nConnection: close\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n",
            payload.len()
        )?;
        stream.write_all(&payload)?;
        Response::read(BufReader::new(stream))
    }

    pub fn request<T: DeserializeOwned>(
        &self,
        method: &str,
        path: &str,
        body: Option<&Value>,
    ) -> Result<T, Error> {
        let mut response = self.connect(method, path, body)?;
        let bytes = response.body()?;
        if !(200..300).contains(&response.status) {
            #[derive(Deserialize)]
            struct Envelope {
                error: ApiError,
            }
            #[derive(Deserialize)]
            struct ApiError {
                code: String,
                message: String,
                state: Option<String>,
            }
            let error: Envelope = serde_json::from_slice(&bytes)?;
            return Err(Error::Api {
                status: response.status,
                code: error.error.code,
                message: error.error.message,
                state: error.error.state,
            });
        }
        Ok(serde_json::from_slice(&bytes)?)
    }

    pub fn events(&self) -> Result<Events, Error> {
        let response = self.connect("GET", "/v1/events", None)?;
        if response.status != 200 {
            return Err(Error::Protocol(format!(
                "Event stream returned HTTP {}",
                response.status
            )));
        }
        Ok(Events::new(response))
    }
}

pub fn segment(value: &str) -> String {
    let mut encoded = String::new();
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || b"-._~".contains(&byte) {
            encoded.push(char::from(byte));
        } else {
            encoded.push_str(&format!("%{byte:02X}"));
        }
    }
    encoded
}
