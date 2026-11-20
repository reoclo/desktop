use super::Share;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TunnelState {
    #[default]
    Down,
    Ready,
    Connecting,
    Active,
    Reconnecting,
    NeedsSignIn,
    ServerOffline,
    QuotaReached,
    Expired,
    ClosedByOperator,
    Forbidden,
    Failed,
    #[serde(other)]
    Unknown,
}

impl TunnelState {
    pub fn label(self) -> &'static str {
        match self {
            Self::Down => "Down",
            Self::Ready => "Ready",
            Self::Connecting => "Connecting",
            Self::Active => "Active",
            Self::Reconnecting => "Reconnecting",
            Self::NeedsSignIn => "Sign in required",
            Self::ServerOffline => "Server offline",
            Self::QuotaReached => "Limit reached",
            Self::Expired => "Expired",
            Self::ClosedByOperator => "Closed by operator",
            Self::Forbidden => "Access refused",
            Self::Failed => "Failed",
            Self::Unknown => "Unknown",
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct TunnelTraffic {
    pub bytes_in: u64,
    pub bytes_out: u64,
    pub streams: u32,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct TunnelView {
    pub name: String,
    pub org: String,
    pub server: String,
    pub server_id: String,
    pub kind: String,
    pub state: TunnelState,
    pub message: String,
    pub action: String,
    pub detail: Option<String>,
    pub enabled: bool,
    pub keep_up: bool,
    pub endpoint: Option<String>,
    pub target: Option<String>,
    pub local_port: Option<u16>,
    pub remote_port: Option<u16>,
    pub local_target: Option<String>,
    pub hostname: Option<String>,
    pub url: Option<String>,
    pub via: Option<String>,
    pub share: Option<Share>,
    pub traffic: TunnelTraffic,
}

impl TunnelView {
    /// Additional context only; the state mark and reason alert already explain the outcome.
    pub fn subtitle(&self) -> Option<&str> {
        let message = self.message.trim();
        let normalized = message.trim_end_matches(['.', '!', '?']).to_lowercase();
        let outcome = normalized
            .split_whitespace()
            .filter(|word| !matches!(*word, "the" | "tunnel" | "is" | "has"))
            .collect::<Vec<_>>()
            .join(" ");
        if normalized.is_empty()
            || outcome == self.state.label().to_lowercase()
            || self
                .detail
                .as_ref()
                .is_some_and(|detail| detail.to_lowercase().contains(&normalized))
        {
            None
        } else {
            Some(message)
        }
    }

    /// Where the tunnel goes: its server, or this Mac for a local route.
    pub fn place(&self) -> &str {
        if self.kind == "local" || self.server.is_empty() {
            "This Mac"
        } else {
            &self.server
        }
    }

    pub fn endpoint(&self) -> &str {
        self.url
            .as_deref()
            .or(self.endpoint.as_deref())
            .unwrap_or("Not listening")
    }
}

impl TunnelState {
    pub fn available(self) -> bool {
        match self {
            Self::Active | Self::Ready => true,
            Self::Down
            | Self::Connecting
            | Self::Reconnecting
            | Self::NeedsSignIn
            | Self::ServerOffline
            | Self::QuotaReached
            | Self::Expired
            | Self::ClosedByOperator
            | Self::Forbidden
            | Self::Failed
            | Self::Unknown => false,
        }
    }
    pub fn failure(self) -> bool {
        match self {
            Self::Failed
            | Self::Forbidden
            | Self::Expired
            | Self::ClosedByOperator
            | Self::NeedsSignIn
            | Self::QuotaReached
            | Self::ServerOffline => true,
            Self::Down
            | Self::Ready
            | Self::Connecting
            | Self::Active
            | Self::Reconnecting
            | Self::Unknown => false,
        }
    }
}
