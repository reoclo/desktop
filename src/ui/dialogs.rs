use super::{State, primitives::*};
use crate::{
    cli::{self, Cli, MIN_VERSION},
    control::segment,
    system::SystemAction,
    theme::*,
};
use rune::{El, col, row, text};
use rune_kit::{Button, Dialog, Kit};

pub fn confirmations(s: &State, k: &Kit, layer: &mut Vec<El>) {
    let Some(modal) = s.modal.as_deref() else {
        return;
    };
    let link_text = s.link_description();
    let (title, description, action, destructive) = match modal {
        "open-link" => (
            "Open this link?",
            link_text.as_deref().unwrap_or_default(),
            "Continue",
            false,
        ),
        "remove" => (
            "Remove tunnel?",
            "This closes the tunnel and removes its saved configuration and hostname.",
            "Remove tunnel",
            true,
        ),
        "stop-terminal" => (
            "Stop this tunnel?",
            "The reoclo process running in your terminal exits and its ports close.",
            "Stop tunnel",
            true,
        ),
        "discard" => (
            "Discard changes?",
            "The tunnel has not been saved.",
            "Discard changes",
            true,
        ),
        "start" => (
            "Start local daemon?",
            "Reoclo will install its background service with the reoclo CLI on this Mac. It starts now and every time you log in. Your saved tunnels remain owned by the daemon.",
            "Start daemon",
            false,
        ),
        "remove-suffix" => (
            "Remove this domain?",
            "Names under it stop getting certificates. The CA is issued again without it, and macOS asks for your password to trust the new one. Remove tunnels and hostnames under it first.",
            "Remove domain",
            true,
        ),
        "relay-on" => (
            "Turn on the port 443 relay?",
            "HTTPS addresses drop the :8443. macOS asks for administrator approval to reinstall the hosts helper. The relay waits while another program, such as a local Caddy, answers on port 443.",
            "Turn on relay",
            false,
        ),
        "relay-off" => (
            "Turn off the port 443 relay?",
            "HTTPS addresses include the daemon's port again. macOS asks for administrator approval to reinstall the hosts helper.",
            "Turn off relay",
            false,
        ),
        "setup" => (
            "Set up managed hosts?",
            "macOS will ask for administrator approval to install the hosts helper. It only manages Reoclo's loopback entries.",
            "Set up helper",
            false,
        ),
        "trust" => (
            "Trust local certificate authority?",
            "The daemon will ask macOS to trust its local CA for Reoclo hostnames.",
            "Trust CA",
            false,
        ),
        "untrust" => (
            "Remove CA trust?",
            "Local HTTPS tunnels will no longer be trusted by browsers.",
            "Untrust CA",
            true,
        ),
        _ => return,
    };
    let modal = modal.to_owned();
    let mut body = col()
        .w_full()
        .gap(LG)
        .padding(LG)
        .child(text(description).font(BODY).color(k.c.text_muted).wrap());
    if let Some(error) = &s.error {
        body = body.child(alert(error, k));
    }
    let confirm = Button::new(action)
        .disabled(s.busy)
        .on_press(move |s: &mut State| match modal.as_str() {
            "remove" => {
                if let Some(t) = s.selected() {
                    s.request("DELETE", &format!("/v1/tunnels/{}", segment(&t.name)), None);
                }
            }
            "stop-terminal" => {
                if let Some(id) = s.stop_target.clone() {
                    s.request(
                        "POST",
                        &format!("/v1/foreground/{}/stop", segment(&id)),
                        None,
                    );
                }
            }
            "discard" => {
                s.modal = None;
                s.return_modal = None;
                s.open_select = None;
            }
            "trust" => s.request("POST", "/v1/ca/trust", None),
            "untrust" => s.request("POST", "/v1/ca/untrust", None),
            "start" => s.run_system(SystemAction::InstallDaemon),
            "open-link" => s.confirm_link(),
            "setup" => {
                let setup = s.helper_setup(s.relay_enabled());
                s.run_system(SystemAction::SetupHosts(setup));
            }
            "remove-suffix" => {
                if let Some(target) = s.suffix_target.clone() {
                    let rest: Vec<String> = s
                        .settings
                        .ca_suffixes
                        .iter()
                        .filter(|suffix| **suffix != target)
                        .cloned()
                        .collect();
                    s.request(
                        "PATCH",
                        "/v1/settings",
                        Some(serde_json::json!({ "ca_suffixes": rest })),
                    );
                }
            }
            "relay-on" | "relay-off" => {
                let setup = s.helper_setup(modal == "relay-on");
                s.run_system(SystemAction::SetupHosts(setup));
            }
            _ => {}
        });
    body = body.child(
        row()
            .w_full()
            .gap(SM)
            .child(spacer())
            .child(
                Button::new("Cancel")
                    .on_press(|s: &mut State| s.cancel_confirmation())
                    .secondary(k)
                    .key("cancel-confirmation"),
            )
            .child(
                if destructive {
                    confirm.destructive(k)
                } else {
                    confirm.primary(k)
                }
                .key("confirm-action"),
            ),
    );
    Dialog::new("confirmation", title)
        .open(true)
        .fit()
        .width(if s.panel { 308.0 } else { 480.0 })
        .click_outside(false)
        .on_close(|s: &mut State| s.cancel_confirmation())
        .child(body)
        .build(k, layer);
}

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
            (SystemAction::StartAtLogin(_), _) => None,
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
