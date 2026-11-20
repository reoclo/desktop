mod client;
mod error;
pub mod install;
mod manifest;
mod restart;
pub mod worker;
pub use client::Client;
pub use error::Error;
pub use manifest::{Manifest, VerifiedArchive, public_key};
pub use restart::finish_if_requested;

pub const LATEST_URL: &str =
    "https://github.com/reoclo/desktop/releases/latest/download/latest.json";
pub const INTERVAL: std::time::Duration = std::time::Duration::from_secs(6 * 60 * 60);

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Phase {
    Disabled(String),
    Idle,
    Checking,
    Available { version: String },
    Ready { version: String, notes: String },
    Installing,
    Failed(String),
}

#[derive(Clone, Debug)]
pub struct Status {
    pub phase: Phase,
    pub last_checked: Option<String>,
    pub automatic: bool,
}

impl Default for Status {
    fn default() -> Self {
        Self {
            phase: Phase::Disabled("Updates require a writable Reoclo.app bundle.".into()),
            last_checked: None,
            automatic: true,
        }
    }
}
