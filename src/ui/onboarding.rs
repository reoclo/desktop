use super::{State, ViewState, links::LinkStep, primitives::*};
use crate::{model::Ca, system::SystemAction, theme::*};
use rune::{El, ScrollSpec, col, el, row, text};
use rune_kit::{Button, Dialog, Kit};

/// Where one Get started step stands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SetupStatus {
    Done,
    Needed,
    Optional,
    /// The step cannot run until the background service does.
    Waiting,
    Checking,
}

impl State {
    pub fn begin_setup(&mut self) {
        self.refresh_service();
        self.request("GET", "/v1/status", None);
        self.modal = Some("get-started".into());
    }

    pub fn finish_setup(&mut self) {
        self.onboarded = true;
        self.modal = None;
        self.save_preferences();
    }

    pub fn refresh_service(&mut self) {
        if let Some(path) = &self.service_plist {
            self.service_installed = path.exists();
        }
    }

    /// The install finished. Without a launch agent path to read, the install is taken at its word.
    pub fn service_installed_ok(&mut self) {
        if self.service_plist.is_some() {
            self.refresh_service();
        } else {
            self.service_installed = true;
        }
    }

    pub fn daemon_running(&self) -> bool {
        !matches!(self.phase, ViewState::Offline | ViewState::Loading)
            && !self.model.status.version.is_empty()
    }

    pub fn signed_in(&self) -> bool {
        self.model.status.signed_in == Some(true)
    }

    pub fn secure_ready(&self) -> bool {
        self.ca_ready() && self.helper_installed()
    }

    pub fn setup_status(&self) -> [SetupStatus; 3] {
        let blocked = if self.phase == ViewState::Loading {
            SetupStatus::Checking
        } else {
            SetupStatus::Waiting
        };
        let running = self.daemon_running();
        [
            if self.phase == ViewState::Loading {
                SetupStatus::Checking
            } else if running && self.service_installed {
                SetupStatus::Done
            } else {
                SetupStatus::Needed
            },
            if !running {
                blocked
            } else if self.signed_in() {
                SetupStatus::Done
            } else {
                SetupStatus::Needed
            },
            if !running {
                blocked
            } else if self.secure_ready() {
                SetupStatus::Done
            } else {
                SetupStatus::Optional
            },
        ]
    }

    /// Trusts the CA, then installs the hosts helper, each only if still needed.
    pub fn secure_next(&mut self) {
        self.securing = true;
        self.busy = false;
        if !self.ca_ready() {
            self.request("POST", "/v1/ca/trust", None);
        } else if !self.helper_installed() {
            let setup = self.helper_setup(self.relay_enabled());
            self.run_system(SystemAction::SetupHosts(setup));
        } else {
            self.securing = false;
            self.secure_finished();
        }
    }

    pub(super) fn secure_after_trust(&mut self) {
        let ca = self.model.status.ca.get_or_insert_with(Ca::default);
        ca.exists = true;
        ca.trusted = true;
        ca.stale = false;
        self.secure_next();
    }

    pub(super) fn secure_after_helper(&mut self) {
        if !self.securing {
            return;
        }
        self.securing = false;
        if self.helper_installed() {
            self.secure_finished();
        } else if self.link_step() == Some(LinkStep::Secure) {
            self.link = None;
        }
    }

    pub(super) fn secure_failed(&mut self, request: &super::Request) {
        if self.securing && request.path == "/v1/ca/trust" {
            self.securing = false;
        }
    }

    pub fn system_failed(&mut self, action: &SystemAction) {
        if self.securing && matches!(action, SystemAction::SetupHosts(_)) {
            self.securing = false;
            if self.link_step() == Some(LinkStep::Secure) {
                self.link = None;
            }
        }
    }

    fn secure_finished(&mut self) {
        if self.link_step() == Some(LinkStep::Secure) {
            self.link_proceed();
        } else {
            self.notice = Some("Secure web addresses are ready.".into());
        }
    }
}

fn mark(status: SetupStatus, k: &Kit) -> El {
    match status {
        SetupStatus::Done => indicator(Tone::Success, "Done", k),
        SetupStatus::Needed => indicator(Tone::Warning, "Needs setup", k),
        SetupStatus::Optional => indicator(Tone::Inactive, "Optional", k),
        SetupStatus::Waiting => indicator(Tone::Inactive, "Needs the background service", k),
        SetupStatus::Checking => indicator(Tone::Info, "Checking", k),
    }
}

fn step(
    k: &Kit,
    title: &str,
    copy: &str,
    status: SetupStatus,
    extra: Vec<El>,
    action: Option<El>,
) -> El {
    let mut body = col()
        .w_full()
        .gap(SM)
        .child(
            row()
                .w_full()
                .gap(MD)
                .center_y()
                .child(
                    text(title)
                        .font(HEADING)
                        .color(k.c.text_base)
                        .truncate()
                        .grow(1.0)
                        .min_w(0.0),
                )
                .child(mark(status, k)),
        )
        .child(text(copy).font(BODY).color(k.c.text_muted).wrap());
    for item in extra {
        body = body.child(item);
    }
    if let Some(action) = action {
        body = body.child(row().child(action));
    }
    body
}

fn rule(k: &Kit) -> El {
    el().h(RULE).w_full().fill(k.c.border_base)
}

pub fn sheet(s: &State, k: &Kit, layer: &mut Vec<El>) {
    if s.modal.as_deref() != Some("get-started") || s.panel {
        return;
    }
    let [service, sign_in, secure] = s.setup_status();
    // The next required step carries the primary action; Done takes it once they are finished.
    let required_done = service == SetupStatus::Done && sign_in == SetupStatus::Done;
    let service_action = (service == SetupStatus::Needed).then(|| {
        Button::new("Install service")
            .disabled(s.busy)
            .on_press(|s: &mut State| s.run_system(SystemAction::InstallDaemon))
            .primary(k)
            .key("setup-install-service")
    });
    let service_waiting = service_action.is_some();
    let waiting_for_code = s.model.auth.user_code.is_some() && s.model.auth.status == "pending";
    let (sign_in_extra, sign_in_action) = if sign_in != SetupStatus::Needed {
        (Vec::new(), None)
    } else if waiting_for_code {
        (device_code(s, k, "setup-"), None)
    } else {
        (
            Vec::new(),
            Some(
                {
                    let sign_in_button = Button::new("Sign in")
                        .disabled(s.busy)
                        .on_press(|s: &mut State| s.request("POST", "/v1/auth/login", None));
                    if service_waiting {
                        sign_in_button.secondary(k)
                    } else {
                        sign_in_button.primary(k)
                    }
                }
                .key("setup-sign-in"),
            ),
        )
    };
    let secure_action = (secure == SetupStatus::Optional).then(|| {
        Button::new("Set up secure addresses")
            .disabled(s.busy)
            .on_press(|s: &mut State| s.secure_next())
            .secondary(k)
            .key("setup-secure")
    });
    let steps = col()
        .w_full()
        .gap(LG)
        .padding(LG)
        .child(
            text("Three quick steps get this Mac ready. You can run them again from Settings.")
                .font(BODY)
                .color(k.c.text_muted)
                .wrap(),
        )
        .child(step(
            k,
            "1. Background service",
            "Reoclo runs your tunnels from a small service on this Mac. Installing it starts it now and again every time you log in.",
            service,
            Vec::new(),
            service_action,
        ))
        .child(rule(k))
        .child(step(
            k,
            "2. Sign in",
            "Sign in with your Reoclo account so this Mac can reach your servers.",
            sign_in,
            sign_in_extra,
            sign_in_action,
        ))
        .child(rule(k))
        .child(step(
            k,
            "3. Secure web addresses",
            "Only needed for HTTPS links. Reoclo trusts its own certificate on this Mac, then adds a small helper that points secure addresses at your computer. macOS asks for your password.",
            secure,
            Vec::new(),
            secure_action,
        ));
    let mut dialog = Dialog::new("get-started", "Get started")
        .open(true)
        .width(560.0)
        .fit()
        .click_outside(false)
        .on_close(|s: &mut State| s.modal = None)
        .child(
            k.scroll_bar(
                col()
                    .w_full()
                    .h(380.0)
                    .scroll(ScrollSpec {
                        stick_bottom: false,
                    })
                    .scroll_edge_fade(LG)
                    .scroll_margin(LG)
                    .key("setup-scroll")
                    .child(steps),
            ),
        );
    if let Some(error) = &s.error {
        dialog = dialog.child(alert(error, k).px(LG).pb(MD));
    }
    dialog
        .child(
            row().w_full().gap(SM).padding(LG).child(spacer()).child(
                if required_done {
                    Button::new("Done")
                        .on_press(|s: &mut State| s.finish_setup())
                        .primary(k)
                } else {
                    Button::new("Later")
                        .on_press(|s: &mut State| s.finish_setup())
                        .secondary(k)
                }
                .key("setup-done"),
            ),
        )
        .build(k, layer);
}
