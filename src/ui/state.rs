use super::links::{Effect, PendingLink};
use crate::{model::*, reducer::Model};
use rune::TextEdit;
use rune_kit::{ComboboxState, PaneState, TableState};
use serde_json::Value;
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
    Hosts,
    Certificates,
    Settings,
}
impl Page {
    pub const ALL: [Self; 4] = [
        Self::Tunnels,
        Self::Hosts,
        Self::Certificates,
        Self::Settings,
    ];
    pub fn name(self) -> &'static str {
        match self {
            Self::Tunnels => "Tunnels",
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

pub struct State {
    pub model: Model,
    pub page: Page,
    pub phase: ViewState,
    pub panel: bool,
    pub sidebar: PaneState,
    pub table: TableState,
    pub outbox: Vec<Request>,
    pub sender: Option<Sender<Request>>,
    pub busy: bool,
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
    /// The notice on screen and the clock time it goes away.
    pub notice_shown: Option<(String, f64)>,
    pub orgs: Vec<Org>,
    pub servers: Vec<Server>,
    pub ports: Vec<Port>,
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
    pub system_runner: Option<Sender<crate::system::SystemAction>>,
    pub window_handle: Option<rune::AppHandle<State>>,
    pub tray: Option<tray_icon::TrayIcon>,
    pub reduced_motion: bool,
    pub menu_bar: MenuBarStyle,
    pub tray_shown: Option<(MenuBarStyle, usize, usize, usize)>,
    pub preferences_path: Option<std::path::PathBuf>,
    /// Get started was finished once.
    pub onboarded: bool,
    /// The launch agent the daemon install writes; its presence means the service starts at login.
    pub service_plist: Option<std::path::PathBuf>,
    pub service_installed: bool,
    /// A Secure web addresses run is trusting the CA and installing the hosts helper.
    pub securing: bool,
    /// A `reoclo://open` link waiting for a confirmation or for the daemon.
    pub link: Option<PendingLink>,
    /// Things a reply decided to do that need the window, such as opening the browser.
    pub effects: Vec<Effect>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            model: Model::default(),
            page: Page::Tunnels,
            phase: ViewState::Loading,
            panel: false,
            sidebar: PaneState::new(192.0).range(160.0, 256.0),
            table: TableState::new(),
            outbox: Vec::new(),
            sender: None,
            busy: false,
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
            notice_shown: None,
            orgs: Vec::new(),
            servers: Vec::new(),
            ports: Vec::new(),
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
            window_handle: None,
            tray: None,
            reduced_motion: false,
            menu_bar: MenuBarStyle::default(),
            tray_shown: None,
            preferences_path: None,
            onboarded: false,
            service_plist: None,
            service_installed: false,
            securing: false,
            link: None,
            effects: Vec::new(),
        }
    }
}

pub fn read_preferences(path: &std::path::Path) -> Preferences {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

fn write_preferences(path: &std::path::Path, saved: &Preferences) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, serde_json::to_string_pretty(saved)?)
}

/// How long a confirmation stays up. It describes a moment, and the state can change after it.
const NOTICE_SECONDS: f64 = 6.0;

impl State {
    /// Runs before every frame. Returns true when a notice went away and the view must redraw.
    pub fn expire_notice(&mut self, clock: f64, tx: &mut rune::Tx) -> bool {
        let Some(notice) = self.notice.clone() else {
            self.notice_shown = None;
            return false;
        };
        match &self.notice_shown {
            Some((shown, due)) if *shown == notice => {
                if clock >= *due {
                    self.notice = None;
                    self.notice_shown = None;
                    return true;
                }
                tx.wake_at(*due);
            }
            _ => {
                let due = clock + NOTICE_SECONDS;
                self.notice_shown = Some((notice, due));
                tx.wake_at(due);
            }
        }
        false
    }

    /// Keep a choice that belongs to this Mac. Fixture and demo runs have no path, and save nothing.
    pub fn save_preferences(&mut self) {
        let Some(path) = self.preferences_path.clone() else {
            return;
        };
        let saved = Preferences {
            menu_bar: self.menu_bar,
            onboarded: self.onboarded,
        };
        if let Err(error) = write_preferences(&path, &saved) {
            self.error = Some(format!("Could not save your preference: {error}"));
        }
    }

    /// Open the Add tunnel form already filled in to reach this target over HTTPS.
    pub fn begin_https(&mut self, org: &str, server: &str, target: &str, name: String) {
        self.org = org.to_owned();
        self.server.clear();
        self.servers.clear();
        self.ports.clear();
        self.begin_add();
        self.kind = "https".into();
        self.via = "reoclo".into();
        self.name = TextEdit::new(name);
        self.target = TextEdit::new(target);
        self.pending_server = Some(server.to_owned());
    }

    pub fn relay_enabled(&self) -> bool {
        self.model
            .status
            .hosts
            .as_ref()
            .is_some_and(|hosts| hosts.relay.enabled)
    }

    /// A reinstall keeps what the helper already allows and adds the trusted domains outside .localhost,
    /// which the helper allows without being told.
    pub fn helper_setup(&self, relay: bool) -> crate::system::HelperSetup {
        let current = self
            .model
            .status
            .hosts
            .as_ref()
            .map(|hosts| hosts.suffixes.clone())
            .unwrap_or_default();
        let mut allow: Vec<String> = current
            .into_iter()
            .chain(self.settings.ca_suffixes.iter().cloned())
            .filter(|suffix| suffix != "localhost" && !suffix.ends_with(".localhost"))
            .collect();
        allow.sort();
        allow.dedup();
        crate::system::HelperSetup {
            relay_port: relay.then_some(self.settings.https_port),
            allow_suffixes: allow,
        }
    }

    pub fn close_pickers(&mut self) {
        self.org_picker.close();
        self.server_picker.close();
        self.port_picker.close();
        self.kind_picker.close();
        self.via_picker.close();
        self.domain_picker.close();
    }

    pub fn helper_installed(&self) -> bool {
        self.model
            .status
            .hosts
            .as_ref()
            .is_some_and(|hosts| hosts.installed)
    }

    /// True when the CA exists and this Mac trusts it, so HTTPS certificates will be accepted.
    pub fn ca_ready(&self) -> bool {
        self.model
            .status
            .ca
            .as_ref()
            .is_some_and(|ca| ca.exists && ca.trusted)
    }

    /// The domains HTTPS hostnames can use, the built-in one first.
    pub fn trusted_domains(&self) -> Vec<String> {
        let mut extra: Vec<String> = self
            .model
            .status
            .ca
            .as_ref()
            .map(|ca| ca.suffixes.clone())
            .unwrap_or_default()
            .into_iter()
            .filter(|suffix| suffix != "reoclo.localhost")
            .collect();
        extra.sort();
        extra.dedup();
        std::iter::once("reoclo.localhost".to_owned())
            .chain(extra)
            .collect()
    }

    /// True when the page has something to list: a saved tunnel, or one in a terminal.
    pub fn has_tunnels(&self) -> bool {
        self.phase == ViewState::Loaded
            || (self.phase == ViewState::Empty && !self.model.foreground.is_empty())
    }

    /// The daemon sends no event when the helper changes, so Hosts asks each time it opens.
    pub fn show_page(&mut self, page: Page) {
        self.page = page;
        self.error = None;
        self.notice = None;
        if page == Page::Hosts {
            self.request("GET", "/v1/hosts", None);
        }
    }

    pub fn request(&mut self, method: &str, path: &str, body: Option<Value>) {
        if self.busy {
            return;
        }
        self.error = None;
        self.notice = None;
        let request = Request {
            method: method.into(),
            path: path.into(),
            body,
        };
        if let Some(sender) = &self.sender {
            match sender.send(request.clone()) {
                Ok(()) => self.busy = true,
                Err(error) => self.error = Some(format!("Control worker stopped: {error}")),
            }
        }
        self.outbox.push(request);
        if self.outbox.len() > 32 {
            self.outbox.remove(0);
        }
    }

    pub fn selected(&self) -> Option<&TunnelView> {
        self.model
            .selected
            .as_ref()
            .and_then(|name| self.model.tunnels.iter().find(|t| &t.name == name))
    }

    pub fn edit_selected(&mut self) {
        if let Some(tunnel) = self.selected().cloned() {
            self.name = TextEdit::new(&tunnel.name);
            self.target = TextEdit::new(
                tunnel
                    .target
                    .as_deref()
                    .or(tunnel.local_target.as_deref())
                    .unwrap_or(""),
            );
            let host = tunnel.hostname.clone().unwrap_or_default();
            let domain = self
                .trusted_domains()
                .into_iter()
                .filter(|d| host == *d || host.ends_with(&format!(".{d}")))
                .max_by_key(String::len)
                .unwrap_or_default();
            self.domain = domain;
            self.hostname = TextEdit::new(host);
            self.local_port = TextEdit::new(
                tunnel
                    .local_port
                    .or(tunnel.remote_port)
                    .unwrap_or(15432)
                    .to_string(),
            );
            self.org = tunnel.org;
            self.server = tunnel.server_id;
            self.kind = tunnel.kind;
            self.via = tunnel.via.unwrap_or_else(|| "reoclo".into());
            self.modal = Some("edit".into());
        }
    }
}
