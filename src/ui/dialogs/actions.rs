use super::*;
use crate::cli::{self, Cli, MIN_VERSION};
use crate::{link::LinkKind, ui::LinkStep};

impl State {
    pub fn confirm_discard(&mut self) {
        self.return_modal = self.modal.take();
        self.modal = Some("discard".into());
        self.open_select = None;
        self.close_pickers();
    }

    pub fn cancel_confirmation(&mut self) {
        if self.modal.as_deref() == Some("open-link") {
            self.link = None;
        }
        self.modal = if self.modal.as_deref() == Some("discard") {
            self.return_modal.take()
        } else {
            None
        };
    }

    pub fn begin_add(&mut self) {
        self.close_pickers();
        self.selected_port = None;
        self.modal = Some("add".into());
        self.name = rune::TextEdit::new("");
        self.hostname = rune::TextEdit::new("");
        self.domain.clear();
        self.error = None;
        if !self.org.is_empty() && self.servers.is_empty() {
            self.request(
                "GET",
                &format!("/v1/orgs/{}/servers", segment(&self.org)),
                None,
            );
        }
    }

    pub fn run_system(&mut self, action: SystemAction) {
        if self.busy {
            return;
        }
        let Some(sender) = self.system_runner.clone() else {
            self.notice = Some("Fixture preview. No system changes were made.".into());
            self.modal = None;
            return;
        };
        let cli = match (&action, &self.cli) {
            (
                SystemAction::StartAtLogin(_)
                | SystemAction::ShowLog(_)
                | SystemAction::ListAgents { .. }
                | SystemAction::ConnectAgents { .. },
                _,
            ) => None,
            (_, Some(Cli::Ready { path, .. })) => Some(path.clone()),
            (_, found) => {
                self.error = Some(cli_problem(found.as_ref()));
                self.system_failed(&action);
                return;
            }
        };
        match sender.send((action, cli)) {
            Ok(()) => {
                self.busy = true;
                self.error = None;
            }
            Err(error) => self.error = Some(format!("System action could not start: {error}")),
        }
    }
}

impl State {
    /// A system action finished on the worker thread.
    pub fn system_finished(&mut self, action: &SystemAction, result: Result<String, String>) {
        self.busy = false;
        if self.agents_finished(action, &result) {
            return;
        }
        match result {
            Ok(message) => {
                self.modal = None;
                if let SystemAction::StartAtLogin(enabled) = action {
                    self.start_at_login = *enabled;
                }
                if *action == SystemAction::InstallDaemon {
                    self.service_installed_ok();
                    self.service_install_finished();
                }
                if matches!(action, SystemAction::SetupHosts(_)) {
                    // The helper's own answer decides what the app reports.
                    self.request("POST", "/v1/hosts/sync", None);
                    self.awaiting_helper = true;
                } else {
                    self.request("GET", "/v1/status", None);
                    self.notice = Some(message);
                }
            }
            Err(message) => {
                self.error = Some(message);
                self.system_failed(action);
            }
        }
    }
}

/// Why a system action cannot run without a usable CLI.
fn cli_problem(found: Option<&Cli>) -> String {
    match found {
        Some(Cli::TooOld { path, version }) => format!(
            "The reoclo CLI at {} is {version}, and this app needs {MIN_VERSION} or later. Run {} and try again.",
            path.display(),
            cli::upgrade_command(path)
        ),
        Some(Cli::Missing) => {
            "No reoclo CLI was found. Install it with brew install reoclo/tap/reoclo and try again."
                .into()
        }
        _ => "Reoclo is still looking for the reoclo CLI. Try again in a moment.".into(),
    }
}

impl State {
    /// What the confirmation dialog says the link will do.
    pub fn link_description(&self) -> Option<String> {
        let pending = self
            .link
            .as_ref()
            .filter(|pending| pending.step == LinkStep::Confirm)?;
        let link = &pending.link;
        let (what, after) = match link.kind {
            LinkKind::Https => (
                format!("a secure web address (HTTPS) for {}", link.target()),
                "It then opens the address in your browser.",
            ),
            LinkKind::Tcp => (
                format!(
                    "a local port on this Mac that forwards to {}",
                    link.target()
                ),
                "It then copies the local address to your clipboard.",
            ),
        };
        Some(format!(
            "This link wants to reach {what} on the server {} in the organization {}. {after} Only continue if you trust where the link came from.",
            link.server, link.org
        ))
    }
}
