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
    pub orgs: Vec<Org>,
    pub servers: Vec<Server>,
    pub ports: Vec<Port>,
    pub org: String,
    pub server: String,
    pub kind: String,
    pub via: String,
    pub open_select: Option<String>,
    pub port_picker: ComboboxState,
    pub selected_port: Option<String>,
    pub name: TextEdit,
    pub target: TextEdit,
    pub hostname: TextEdit,
    pub local_port: TextEdit,
    pub idle: TextEdit,
    pub https_port: TextEdit,
    pub approvals_required: bool,
    pub start_at_login: bool,
    pub appearance: String,
    pub settings: Settings,
    pub system_runner: Option<Sender<crate::system::SystemAction>>,
    pub window_handle: Option<rune::AppHandle<State>>,
    pub tray: Option<tray_icon::TrayIcon>,
    pub reduced_motion: bool,
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
            orgs: Vec::new(),
            servers: Vec::new(),
            ports: Vec::new(),
            org: String::new(),
            server: String::new(),
            kind: "https".into(),
            via: "reoclo".into(),
            open_select: None,
            port_picker: ComboboxState::default(),
            selected_port: None,
            name: TextEdit::new(""),
            target: TextEdit::new("127.0.0.1:3000"),
            hostname: TextEdit::new(""),
            local_port: TextEdit::new("15432"),
            idle: TextEdit::new("600"),
            https_port: TextEdit::new("8443"),
            approvals_required: true,
            start_at_login: false,
            appearance: "system".into(),
            settings: Settings::default(),
            system_runner: None,
            window_handle: None,
            tray: None,
            reduced_motion: false,
        }
    }
}

impl State {
    pub fn request(&mut self, method: &str, path: &str, body: Option<Value>) {
        if self.busy {
            return;
        }
        self.error = None;
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
            self.hostname = TextEdit::new(tunnel.hostname.as_deref().unwrap_or(""));
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
