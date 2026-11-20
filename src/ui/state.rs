use super::links::{Effect, PendingLink};
use crate::{cli::Cli, model::*, reducer::Model};
use rune::TextEdit;
use rune_kit::{ComboboxState, PaneState, TableState};
use serde_json::Value;
mod actions;
pub use actions::read_preferences;
use std::sync::mpsc::Sender;

#[derive(Clone, Debug)]
pub struct Request {
    pub method: String,
    pub path: String,
    pub body: Option<Value>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Page {
    Tunnels,
    Logs,
    Hosts,
    Certificates,
    Settings,
}
impl Page {
    pub const ALL: [Self; 5] = [
        Self::Tunnels,
        Self::Logs,
        Self::Hosts,
        Self::Certificates,
        Self::Settings,
    ];
    pub fn name(self) -> &'static str {
        match self {
            Self::Tunnels => "Tunnels",
            Self::Logs => "Logs",
            Self::Hosts => "Hosts",
            Self::Certificates => "Certificates",
            Self::Settings => "Settings",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ViewState {
    Loading,
    Loaded,
    Empty,
    Offline,
    SignedOut,
    Error,
}

/// Where the scan of the chosen server's listening ports stands.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum PortsScan {
    #[default]
    Idle,
    Scanning,
    Loaded,
    Failed(String),
}

pub struct State {
    pub logs: super::logs::LogView,
    pub model: Model,
    pub page: Page,
    pub phase: ViewState,
    pub panel: bool,
    pub sidebar: PaneState,
    pub table: TableState,
    pub outbox: Vec<Request>,
    pub sender: Option<Sender<Request>>,
    pub busy: bool,
    pub pending: std::collections::BTreeSet<String>,
    pub compact: bool,
    pub viewport_width: f32,
    pub graph_width: f32,
    /// The Tunnels page's scrolling height, and the height of what sits above its table.
    pub tunnels_space: [f32; 2],
    pub clock_epoch: time::OffsetDateTime,
    pub clock_seconds: Option<f64>,
    pub socket_path: std::path::PathBuf,
    pub ca_path: Option<std::path::PathBuf>,
    pub ca_path_error: Option<String>,
    pub error: Option<String>,
    pub notice: Option<String>,
    pub tray_error: Option<String>,
    pub modal: Option<String>,
    pub return_modal: Option<String>,
    pub stop_target: Option<String>,
    /// The allowed suffix the remove confirmation is about.
    pub suffix_target: Option<String>,
    /// A suffix was removed; the next trust issues the CA without it.
    pub reissuing: bool,
    /// The server to select once the Add tunnel form has its server list.
    pub pending_server: Option<String>,
    /// The helper install finished; the next status decides what the app reports.
    pub awaiting_helper: bool,
    /// The reply to a CA trust request was lost; the next status says whether the CA got trusted anyway.
    pub trust_check: Option<(super::Request, String)>,
    /// The notice on screen and the clock time it goes away.
    pub notice_shown: Option<(String, f64)>,
    pub orgs: Vec<Org>,
    pub servers: Vec<Server>,
    pub ports: Vec<Port>,
    pub ports_scan: PortsScan,
    pub org: String,
    pub server: String,
    pub kind: String,
    pub via: String,
    pub open_select: Option<String>,
    pub org_picker: ComboboxState,
    pub server_picker: ComboboxState,
    pub port_picker: ComboboxState,
    pub kind_picker: ComboboxState,
    pub via_picker: ComboboxState,
    pub selected_port: Option<String>,
    pub name: TextEdit,
    pub target: TextEdit,
    /// The subdomain, or a whole hostname on the chosen domain.
    pub hostname: TextEdit,
    /// The trusted domain an HTTPS hostname sits on. Empty means reoclo.localhost.
    pub domain: String,
    pub domain_picker: ComboboxState,
    pub local_port: TextEdit,
    pub idle: TextEdit,
    pub trust_domain: TextEdit,
    pub new_host: TextEdit,
    pub https_port: TextEdit,
    pub approvals_required: bool,
    pub start_at_login: bool,
    pub appearance: String,
    pub settings: Settings,
    pub system_runner: Option<Sender<(crate::system::SystemAction, Option<std::path::PathBuf>)>>,
    /// The reoclo CLI system actions run. None until the first search finishes.
    pub cli: Option<Cli>,
    /// Starts a search, given the executable the daemon reports.
    pub cli_finder: Option<Sender<Option<String>>>,
    /// The daemon executable the last search was for, so a routine status does not search again.
    pub cli_searched: Option<Option<String>>,
    pub window_handle: Option<rune::AppHandle<State>>,
    pub tray: Option<tray_icon::TrayIcon>,
    pub reduced_motion: bool,
    pub menu_bar: MenuBarStyle,
    pub tray_shown: Option<(MenuBarStyle, crate::tray::Counts)>,
    pub preferences_path: Option<std::path::PathBuf>,
    /// Get started was finished once.
    pub onboarded: bool,
    /// The relay setting a running helper last reported. Saved with the preferences.
    pub relay_choice: bool,
    /// The launch agent the daemon install writes; its presence means the service starts at login.
    pub service_plist: Option<std::path::PathBuf>,
    pub service_installed: bool,
    /// A Secure web addresses run is trusting the CA and installing the hosts helper.
    pub securing: bool,
    /// A `reoclo://open` link waiting for a confirmation or for the daemon.
    pub link: Option<PendingLink>,
    /// The first-run screens replace the whole window until Done. `first_run_step` says which one.
    pub first_run: bool,
    /// Sign-in and setup progress for the first-run screens.
    pub flow: super::setup_flow::Flow,
    /// What the agent command install last reported. None until it has run.
    pub command: Option<crate::system::command_path::CommandState>,
    /// Where the app links its CLI from. None outside an app bundle.
    pub command_paths: Option<crate::system::command_path::CommandPaths>,
    /// Connect your agents: the coding agents on this Mac and what registering Reoclo did.
    pub agents: super::AgentsStep,
    pub command_worker: Option<Sender<crate::system::worker::CommandRequest>>,
    /// Things a reply decided to do that need the window, such as opening the browser.
    pub effects: Vec<Effect>,
    pub updates: crate::update::Status,
    pub updater: Option<Sender<crate::update::worker::Command>>,
    pub updater_started: bool,
    pub updater_first_frame: Option<f64>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            logs: super::logs::LogView::default(),
            model: Model::default(),
            page: Page::Tunnels,
            phase: ViewState::Loading,
            panel: false,
            sidebar: PaneState::new(192.0).range(160.0, 256.0),
            table: TableState::new(),
            outbox: Vec::new(),
            sender: None,
            busy: false,
            pending: Default::default(),
            compact: false,
            viewport_width: 1100.0,
            graph_width: 800.0,
            tunnels_space: [0.0; 2],
            clock_epoch: time::OffsetDateTime::now_utc(),
            clock_seconds: None,
            socket_path: Default::default(),
            ca_path: None,
            ca_path_error: None,
            error: None,
            notice: None,
            tray_error: None,
            modal: None,
            return_modal: None,
            stop_target: None,
            suffix_target: None,
            reissuing: false,
            pending_server: None,
            awaiting_helper: false,
            trust_check: None,
            notice_shown: None,
            orgs: Vec::new(),
            servers: Vec::new(),
            ports: Vec::new(),
            ports_scan: PortsScan::Idle,
            org: String::new(),
            server: String::new(),
            kind: "https".into(),
            via: "reoclo".into(),
            open_select: None,
            org_picker: ComboboxState::default(),
            server_picker: ComboboxState::default(),
            port_picker: ComboboxState::default(),
            kind_picker: ComboboxState::default(),
            via_picker: ComboboxState::default(),
            selected_port: None,
            name: TextEdit::new(""),
            target: TextEdit::new("127.0.0.1:3000"),
            hostname: TextEdit::new(""),
            domain: String::new(),
            domain_picker: ComboboxState::default(),
            local_port: TextEdit::new("15432"),
            idle: TextEdit::new("600"),
            trust_domain: TextEdit::new(""),
            new_host: TextEdit::new(""),
            https_port: TextEdit::new("8443"),
            approvals_required: true,
            start_at_login: false,
            appearance: "system".into(),
            settings: Settings::default(),
            system_runner: None,
            cli: None,
            cli_finder: None,
            cli_searched: None,
            window_handle: None,
            tray: None,
            reduced_motion: false,
            menu_bar: MenuBarStyle::default(),
            tray_shown: None,
            preferences_path: None,
            onboarded: false,
            relay_choice: false,
            service_plist: None,
            service_installed: false,
            securing: false,
            link: None,
            first_run: false,
            flow: Default::default(),
            command: None,
            command_paths: None,
            agents: Default::default(),
            command_worker: None,
            effects: Vec::new(),
            updates: crate::update::Status::default(),
            updater: None,
            updater_started: false,
            updater_first_frame: None,
        }
    }
}
