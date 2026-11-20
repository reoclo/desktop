mod tunnel;
use serde::{Deserialize, Serialize};
pub use tunnel::{TunnelState, TunnelTraffic, TunnelView};

/// What the menu bar item shows.
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum MenuBarStyle {
    #[default]
    Icon,
    IconAndName,
    IconAndCount,
    IconAndMeter,
}

impl MenuBarStyle {
    pub const ALL: [Self; 4] = [
        Self::Icon,
        Self::IconAndName,
        Self::IconAndCount,
        Self::IconAndMeter,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Icon => "Icon",
            Self::IconAndName => "Icon and name",
            Self::IconAndCount => "Icon and count",
            Self::IconAndMeter => "Icon and meter",
        }
    }

    pub fn from_label(label: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|style| style.label() == label)
    }
}

/// Choices that belong to this Mac's app rather than the daemon.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(default)]
pub struct Preferences {
    pub menu_bar: MenuBarStyle,
    /// The Get started screen was finished once, so it no longer opens on launch.
    pub onboarded: bool,
    /// The port 443 relay setting a running helper last reported, kept for a reinstall while the helper is down.
    pub relay: bool,
    pub automatic_updates: bool,
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            menu_bar: MenuBarStyle::default(),
            onboarded: false,
            relay: false,
            automatic_updates: true,
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(default)]
pub struct ForegroundForward {
    pub kind: String,
    pub proto: String,
    pub listen: String,
    pub target: String,
}

/// A tunnel running in a terminal, reported by the daemon.
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(default)]
pub struct ForegroundTunnel {
    pub id: String,
    pub pid: u32,
    pub org: String,
    pub server: String,
    pub state: TunnelState,
    pub started_at: String,
    pub forwards: Vec<ForegroundForward>,
    pub traffic: Option<TunnelTraffic>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct ForegroundList {
    pub tunnels: Vec<ForegroundTunnel>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct Share {
    pub tailnet: bool,
    pub url: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct TunnelList {
    pub tunnels: Vec<TunnelView>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct Status {
    pub version: String,
    /// The CLI binary the daemon runs from. Daemons before 0.104.0 do not say.
    pub executable: Option<String>,
    pub pid: u32,
    pub uptime_seconds: u64,
    pub signed_in: Option<bool>,
    pub https: Option<Https>,
    pub ca: Option<Ca>,
    pub hosts: Option<Hosts>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct Https {
    pub port: u16,
    pub listening: bool,
    pub error: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct Ca {
    pub exists: bool,
    pub trusted: bool,
    /// The allowed suffixes changed since this CA was issued; trusting issues a new one.
    pub stale: bool,
    pub not_after: Option<String>,
    pub suffixes: Vec<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct Hosts {
    pub installed: bool,
    /// The helper's version. Helpers before 0.103.1 do not say.
    pub version: Option<String>,
    pub suffixes: Vec<String>,
    pub entries: usize,
    pub relay: Relay,
    pub error: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct Relay {
    pub enabled: bool,
    pub listening: bool,
    pub port: Option<u16>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct HostEntry {
    pub hostname: String,
    pub source: String,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct HostsList {
    pub entries: Vec<HostEntry>,
    pub helper: Hosts,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct Settings {
    pub idle_timeout_seconds: u32,
    pub https_port: u16,
    pub agent_approvals: String,
    pub ca_suffixes: Vec<String>,
    pub manual_hosts: Vec<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            idle_timeout_seconds: 600,
            https_port: 8443,
            agent_approvals: "required".into(),
            ca_suffixes: Vec::new(),
            manual_hosts: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct Org {
    pub id: String,
    pub slug: String,
    pub name: String,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct Server {
    pub id: String,
    pub name: String,
    pub slug: String,
    pub connection_type: String,
    pub status: String,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct Port {
    pub port: u16,
    pub protocol: String,
    pub bind_address: String,
    pub process: String,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct Approval {
    pub id: String,
    pub agent: String,
    pub action: String,
    pub description: String,
    #[serde(with = "time::serde::rfc3339::option")]
    pub expires_at: Option<time::OffsetDateTime>,
    pub status: String,
    pub error: Option<ApprovalError>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ApprovalError {
    pub code: String,
    pub message: String,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct AuthEvent {
    pub user_code: Option<String>,
    pub verification_uri: Option<String>,
    pub verification_uri_complete: Option<String>,
    pub message: Option<String>,
    pub status: String,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct OrgList {
    pub orgs: Vec<Org>,
}
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct ServerList {
    pub servers: Vec<Server>,
}
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct PortList {
    pub ports: Vec<Port>,
}
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct ApprovalList {
    pub approvals: Vec<Approval>,
}
