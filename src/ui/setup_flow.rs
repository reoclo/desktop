//! What the first-run screens read: signing in on a Mac with nothing installed, and the setup steps.
mod steps;
use super::{Request, State, ViewState, links::Effect};
use crate::system::SystemAction;
pub use steps::{SetupStep, StepState};

/// How long the daemon has to answer after its service was installed.
pub const DAEMON_WAIT_SECONDS: f64 = 20.0;

/// Where a sign-in started from the app stands.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum LoginStep {
    #[default]
    Idle,
    /// The background service is being installed with the CLI the app carries.
    InstallingService,
    /// The service is installed and the daemon has not answered yet.
    WaitingForDaemon,
    /// The daemon was asked for a device code.
    RequestingCode,
    /// The code is on screen and the browser was opened once.
    WaitingForApproval,
    SignedIn,
    Failed(String),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Resume {
    Login,
    Setup,
}

/// The wait for the daemon after a service install. The clock starts on the first frame that sees it.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
enum DaemonWait {
    #[default]
    Idle,
    Armed,
    Until(f64),
}

#[derive(Clone, Debug, Default)]
pub struct Flow {
    pub(super) login: LoginStep,
    /// What to do once the daemon answers after a service install.
    resume: Option<Resume>,
    daemon_wait: DaemonWait,
    pub(super) service: StepState,
    pub(super) certificate: StepState,
    pub(super) helper: StepState,
    pub(super) pairing_skipped: bool,
    /// The agent command install: running while the worker links it, failed with its reason.
    pub(super) tools: StepState,
    pub(super) tools_done: bool,
}

impl State {
    pub fn login_step(&self) -> &LoginStep {
        &self.flow.login
    }

    /// Starts the screens over, as Run setup again does.
    pub(super) fn reset_flow(&mut self) {
        self.flow = Flow::default();
        self.agents = Default::default();
    }

    /// Signs in through the daemon, installing its service first when nothing is running.
    pub fn begin_login(&mut self) {
        if self.phase == ViewState::Offline {
            self.flow.login = LoginStep::InstallingService;
            self.install_service(Resume::Login);
        } else {
            self.request_login();
        }
    }

    pub fn cancel_login(&mut self) {
        self.flow.login = LoginStep::Idle;
        self.flow.resume = None;
        self.flow.daemon_wait = DaemonWait::Idle;
        self.model.auth = Default::default();
    }

    fn request_login(&mut self) {
        self.flow.login = LoginStep::RequestingCode;
        self.request("POST", "/v1/auth/login", None);
    }

    fn install_service(&mut self, resume: Resume) {
        self.flow.resume = Some(resume);
        self.flow.service = StepState::Running;
        self.run_system(SystemAction::InstallDaemon);
    }

    pub(super) fn service_install_finished(&mut self) {
        if self.flow.resume.is_some() {
            self.flow.daemon_wait = DaemonWait::Armed;
        }
        if self.flow.login == LoginStep::InstallingService {
            self.flow.login = LoginStep::WaitingForDaemon;
        }
    }

    /// The daemon answered a status request, so whatever waited for it can go on.
    pub(super) fn daemon_answered(&mut self) {
        self.flow.daemon_wait = DaemonWait::Idle;
        match self.flow.resume.take() {
            Some(Resume::Login) => self.request_login(),
            Some(Resume::Setup) => {
                self.flow.service = StepState::Needed;
                self.advance_setup();
            }
            None => {}
        }
    }

    /// Runs every frame. Returns true when the wait ran out and the screen must redraw.
    pub(super) fn expire_daemon_wait(&mut self, clock: f64, tx: &mut rune::Tx) -> bool {
        match self.flow.daemon_wait {
            DaemonWait::Idle => false,
            DaemonWait::Armed => {
                self.flow.daemon_wait = DaemonWait::Until(clock + DAEMON_WAIT_SECONDS);
                tx.wake_at(clock + DAEMON_WAIT_SECONDS);
                false
            }
            DaemonWait::Until(due) if clock >= due => {
                self.daemon_wait_expired();
                true
            }
            DaemonWait::Until(due) => {
                tx.wake_at(due);
                false
            }
        }
    }

    fn daemon_wait_expired(&mut self) {
        let message = "Reoclo's background service did not start in time. Try again.".to_owned();
        self.flow.daemon_wait = DaemonWait::Idle;
        self.flow.resume = None;
        self.flow.service = StepState::Failed(message.clone());
        if self.flow.login == LoginStep::WaitingForDaemon {
            self.flow.login = LoginStep::Failed(message);
        }
    }

    /// The daemon sent the device code. The browser opens once per sign-in.
    pub(super) fn login_code_arrived(&mut self) {
        if self.flow.login != LoginStep::RequestingCode {
            return;
        }
        self.flow.login = LoginStep::WaitingForApproval;
        let auth = &self.model.auth;
        let url = auth
            .verification_uri_complete
            .clone()
            .or_else(|| auth.verification_uri.clone());
        if let Some(url) = url {
            self.effects.push(Effect::OpenUrl(url));
        }
    }

    /// The sign-in finished, so the setup steps start.
    pub(super) fn login_approved(&mut self) {
        if matches!(
            self.flow.login,
            LoginStep::RequestingCode | LoginStep::WaitingForApproval
        ) {
            self.flow.login = LoginStep::SignedIn;
            self.start_setup();
        }
    }

    pub(super) fn login_refused(&mut self) {
        let message = self
            .model
            .auth
            .message
            .clone()
            .unwrap_or_else(|| "Sign-in did not finish.".into());
        self.flow.login = LoginStep::Failed(message);
    }

    /// A request a step was waiting on failed, and its message is already in `error`.
    pub(super) fn flow_failed(&mut self, request: &Request) {
        let message = self.error.clone().unwrap_or_default();
        match request.path.as_str() {
            "/v1/auth/login" if self.flow.login == LoginStep::RequestingCode => {
                self.flow.login = LoginStep::Failed(message);
            }
            "/v1/ca/trust" if self.flow.certificate == StepState::Running => {
                self.flow.certificate = StepState::Failed(message);
            }
            "/v1/hosts/sync" if self.flow.helper == StepState::Running => {
                self.flow.helper = StepState::Failed(message);
            }
            _ => {}
        }
    }

    /// A system action a step was waiting on failed, and its message is already in `error`.
    pub(super) fn flow_system_failed(&mut self, action: &SystemAction) {
        let message = self.error.clone().unwrap_or_default();
        match action {
            SystemAction::InstallDaemon => {
                self.flow.resume = None;
                self.flow.daemon_wait = DaemonWait::Idle;
                self.flow.service = StepState::Failed(message.clone());
                if matches!(
                    self.flow.login,
                    LoginStep::InstallingService | LoginStep::WaitingForDaemon
                ) {
                    self.flow.login = LoginStep::Failed(message);
                }
            }
            SystemAction::SetupHosts(_) if self.flow.helper == StepState::Running => {
                self.flow.helper = StepState::Failed(message);
            }
            _ => {}
        }
    }
}
