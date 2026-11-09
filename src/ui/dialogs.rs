use super::State;
use crate::{control::segment, system::SystemAction, theme::*};
use rune::{El, col, row, text};
use rune_kit::{Button, ButtonVariant, Dialog, Kit};

pub fn confirmations(s: &State, k: &Kit, layer: &mut Vec<El>) {
    let Some(modal) = s.modal.as_deref() else {
        return;
    };
    let (title, description, action, destructive) = match modal {
        "remove" => (
            "Remove tunnel?",
            "This closes the tunnel and removes its saved configuration and hostname.",
            "Remove tunnel",
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
            "Reoclo will start its bundled CLI in the background. Your saved tunnels remain owned by the daemon.",
            "Start daemon",
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
        .gap(LG)
        .padding(LG)
        .child(text(description).font(BODY).color(k.c.text_muted).wrap());
    if let Some(error) = &s.error {
        body = body.child(
            text(error)
                .role("alert")
                .font(BODY)
                .color(k.c.danger.fg)
                .wrap(),
        );
    }
    body = body.child(
        row()
            .gap(SM)
            .child(
                Button::new("Cancel")
                    .on_press(|s: &mut State| s.cancel_confirmation())
                    .build(k)
                    .px(0.0)
                    .key("cancel-confirmation"),
            )
            .child(
                Button::new(action)
                    .variant(if destructive {
                        ButtonVariant::Danger
                    } else {
                        ButtonVariant::Contrast
                    })
                    .disabled(s.busy)
                    .on_press(move |s: &mut State| match modal.as_str() {
                        "remove" => {
                            if let Some(t) = s.selected() {
                                s.request(
                                    "DELETE",
                                    &format!("/v1/tunnels/{}", segment(&t.name)),
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
                        "start" => s.run_system(SystemAction::StartDaemon),
                        "setup" => s.run_system(SystemAction::SetupHosts),
                        _ => {}
                    })
                    .build(k)
                    .key("confirm-action"),
            ),
    );
    Dialog::new("confirmation", title)
        .open(true)
        .fit()
        .width(480.0)
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
        self.port_picker.close();
    }

    pub fn cancel_confirmation(&mut self) {
        self.modal = if self.modal.as_deref() == Some("discard") {
            self.return_modal.take()
        } else {
            None
        };
    }

    pub fn begin_add(&mut self) {
        self.port_picker.close();
        self.selected_port = None;
        self.modal = Some("add".into());
        self.name = rune::TextEdit::new("");
        self.hostname = rune::TextEdit::new("");
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
        if let Some(sender) = &self.system_runner {
            match sender.send(action) {
                Ok(()) => {
                    self.busy = true;
                    self.error = None;
                }
                Err(error) => self.error = Some(format!("System action could not start: {error}")),
            }
        } else {
            self.notice = Some("Fixture preview. No system changes were made.".into());
            self.modal = None;
        }
    }
}
