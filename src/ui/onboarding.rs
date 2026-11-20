mod view;
use super::modal::Dialog;
use super::{State, ViewState, links::LinkStep, primitives::*};
use crate::{model::Ca, system::SystemAction, theme::*};
use rune::{El, ScrollSpec, col, el, row, text};
use rune_kit::{Button, Kit};
pub use view::sheet;

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
