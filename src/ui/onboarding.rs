use super::{State, ViewState, links::LinkStep};
use crate::{model::Ca, system::SystemAction};

impl State {
    /// Opens the first-run screens on a fresh flow, as Run setup again does.
    pub fn begin_setup(&mut self) {
        self.refresh_service();
        self.request("GET", "/v1/status", None);
        self.reset_flow();
        self.first_run = true;
        self.modal = None;
    }

    /// Done on the last screen: the setup is finished once, and a link that waited for it opens now.
    pub fn finish_setup(&mut self) {
        self.onboarded = true;
        self.first_run = false;
        self.save_preferences();
        if self.link_step() == Some(LinkStep::Confirm) {
            self.confirm_link();
        }
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

    pub(super) fn mark_ca_trusted(&mut self) {
        let ca = self.model.status.ca.get_or_insert_with(Ca::default);
        ca.exists = true;
        ca.trusted = true;
        ca.stale = false;
    }

    pub(super) fn secure_after_trust(&mut self) {
        self.mark_ca_trusted();
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
        self.flow_system_failed(action);
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
