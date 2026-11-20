use super::{Page, PortsScan, Request, State, ViewState};
use crate::{
    cli::{self, Cli},
    control::segment,
    model::*,
};
use rune::TextEdit;
use serde_json::Value;

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
    /// A launch with the choices saved on this Mac. Until Done is pressed once, it opens on first run.
    pub fn launch(preferences: &Preferences) -> Self {
        Self {
            menu_bar: preferences.menu_bar,
            relay_choice: preferences.relay,
            onboarded: preferences.onboarded,
            first_run: !preferences.onboarded,
            updates: crate::update::Status {
                automatic: preferences.automatic_updates,
                ..Default::default()
            },
            ..Self::default()
        }
    }

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
            relay: self.relay_choice,
            automatic_updates: self.updates.automatic,
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
        self.ports_scan = PortsScan::Idle;
        self.begin_add();
        self.kind = "https".into();
        self.via = "reoclo".into();
        self.name = TextEdit::new(name);
        self.target = TextEdit::new(target);
        self.pending_server = Some(server.to_owned());
    }

    /// Asks the daemon to scan the chosen server's listening ports.
    pub fn scan_ports(&mut self) {
        self.ports.clear();
        self.selected_port = None;
        self.ports_scan = PortsScan::Scanning;
        let path = format!(
            "/v1/orgs/{}/servers/{}/ports",
            segment(&self.org),
            segment(&self.server)
        );
        self.request("GET", &path, None);
    }

    /// Search for the CLI when the daemon reports a different executable, or always when `force`.
    pub fn find_cli(&mut self, force: bool) {
        let daemon = self.model.status.executable.clone();
        if !force && self.cli_searched.as_ref() == Some(&daemon) {
            return;
        }
        if let Some(finder) = &self.cli_finder
            && finder.send(daemon.clone()).is_ok()
        {
            self.cli_searched = Some(daemon);
        }
    }

    fn cli_version(&self) -> Option<&str> {
        match &self.cli {
            Some(Cli::Ready { version, .. }) => Some(version),
            _ => None,
        }
    }

    /// The daemon runs an older version than the CLI, which is what an upgrade leaves until a restart.
    pub fn daemon_behind(&self) -> bool {
        self.daemon_running()
            && self
                .cli_version()
                .is_some_and(|cli| cli::older(&self.model.status.version, cli))
    }

    /// The installed hosts helper is older than the CLI. Only reinstalling updates its root-owned copy.
    pub fn helper_behind(&self) -> Option<String> {
        let helper = self.model.status.hosts.as_ref().filter(|h| h.installed)?;
        let version = helper.version.clone()?;
        cli::older(&version, self.cli_version()?).then_some(version)
    }

    /// The relay setting of the running helper. While the helper is down, the one it last reported,
    /// so reinstalling it does not turn the relay off.
    pub fn relay_enabled(&self) -> bool {
        match self.model.status.hosts.as_ref() {
            Some(hosts) if hosts.installed => hosts.relay.enabled,
            _ => self.relay_choice,
        }
    }

    /// Keep the relay setting of a running helper.
    pub fn remember_relay(&mut self) {
        if let Some(hosts) = self
            .model
            .status
            .hosts
            .as_ref()
            .filter(|hosts| hosts.installed)
            && hosts.relay.enabled != self.relay_choice
        {
            self.relay_choice = hosts.relay.enabled;
            self.save_preferences();
        }
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
        !self.model.tunnels.is_empty()
            || self.phase == ViewState::Loaded
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
        // The CLI can be upgraded while the app runs.
        if page == Page::Settings {
            self.find_cli(true);
        }
    }

    pub fn request(&mut self, method: &str, path: &str, body: Option<Value>) {
        if self.pending.contains(path) {
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
                Ok(()) => {
                    self.pending.insert(path.into());
                }
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
}
