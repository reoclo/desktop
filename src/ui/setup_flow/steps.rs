//! The setup steps after sign-in, and which first-run screen the plumbing state calls for.
use super::LoginStep;
use crate::{
    cli::Cli,
    system::{SystemAction, command_path::CommandState, worker::CommandRequest},
    ui::{FirstRun, LinkStep, State},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SetupStep {
    Service,
    Certificate,
    Helper,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum StepState {
    #[default]
    Needed,
    /// The next macOS prompt is about to appear. The screen explains it and waits for Continue.
    Explaining,
    Running,
    /// The step did not finish. The screen goes back to its explanation and shows why.
    Failed(String),
    Done,
}

impl State {
    pub fn setup_step(&self, step: SetupStep) -> StepState {
        let (done, stored) = match step {
            SetupStep::Service => (
                self.daemon_running() && self.service_installed,
                &self.flow.service,
            ),
            SetupStep::Certificate => (self.ca_ready(), &self.flow.certificate),
            SetupStep::Helper => (self.helper_installed(), &self.flow.helper),
        };
        if done {
            StepState::Done
        } else {
            stored.clone()
        }
    }

    /// Starts the setup steps over, installing the service first when it is not running.
    pub fn start_setup(&mut self) {
        self.flow.service = StepState::Needed;
        self.flow.certificate = StepState::Needed;
        self.flow.helper = StepState::Needed;
        if self.setup_step(SetupStep::Service) == StepState::Done {
            self.advance_setup();
        } else {
            self.install_service(super::Resume::Setup);
        }
    }

    /// Stops at the next step that raises a macOS prompt, so the screen can explain it first.
    pub(super) fn advance_setup(&mut self) {
        if !self.ca_ready() {
            self.flow.certificate = StepState::Explaining;
        } else if !self.helper_installed() {
            self.flow.helper = StepState::Explaining;
        }
    }

    /// Continue on an explanation screen: this is what raises the macOS prompt.
    pub fn continue_setup(&mut self) {
        if !self.ca_ready() {
            self.flow.certificate = StepState::Running;
            self.request("POST", "/v1/ca/trust", None);
        } else if !self.helper_installed() {
            self.flow.helper = StepState::Running;
            let setup = self.helper_setup(true);
            self.run_system(SystemAction::SetupHosts(setup));
        }
    }

    pub fn skip_pairing(&mut self) {
        self.flow.pairing_skipped = true;
    }

    pub(in crate::ui) fn certificate_trusted(&mut self) {
        if self.flow.certificate == StepState::Running {
            self.mark_ca_trusted();
            self.advance_setup();
        }
    }

    pub(in crate::ui) fn helper_answered(&mut self) {
        if self.flow.helper == StepState::Running && !self.helper_installed() {
            self.flow.helper = StepState::Failed(self.error.clone().unwrap_or_default());
        }
    }

    /// Links the bundled CLI where an agent's shell finds it. The answer lands in `command`.
    pub fn install_command(&mut self) {
        self.ask_command(CommandRequest::Install);
    }

    /// Removes the link and the PATH block the app wrote.
    pub fn remove_command(&mut self) {
        self.ask_command(CommandRequest::Remove);
    }

    fn ask_command(&mut self, request: CommandRequest) {
        match &self.command_worker {
            Some(worker) if worker.send(request).is_ok() => {
                self.flow.tools = StepState::Running;
            }
            _ => self.notice = Some("Fixture preview. No system changes were made.".into()),
        }
    }

    pub fn command_finished(&mut self, state: CommandState) {
        self.flow.tools = match &state {
            CommandState::Failed(message) => StepState::Failed(message.clone()),
            _ => StepState::Needed,
        };
        self.command = Some(state);
    }

    /// Continue or Skip on the tools screen.
    pub fn finish_tools(&mut self) {
        self.flow.tools_done = true;
    }

    /// The first-run screen the plumbing state calls for. Nothing here is stored, so a screen
    /// can only show what the daemon, the system and the worker have reported.
    pub fn first_run_step(&self) -> FirstRun {
        if !(self.signed_in() && self.daemon_running()) {
            return match self.flow.login {
                LoginStep::Idle | LoginStep::Failed(_)
                    if self.link_step() == Some(LinkStep::Confirm) =>
                {
                    FirstRun::Link
                }
                LoginStep::Idle | LoginStep::Failed(_) => FirstRun::Welcome,
                LoginStep::InstallingService
                | LoginStep::WaitingForDaemon
                | LoginStep::RequestingCode
                | LoginStep::WaitingForApproval
                | LoginStep::SignedIn => FirstRun::Waiting,
            };
        }
        if !self.flow.pairing_skipped && !self.secure_ready() {
            return if self.ca_ready() {
                FirstRun::PairingHelper
            } else {
                FirstRun::Pairing
            };
        }
        if !self.tools_resolved() {
            return if self.another_command_in_charge() {
                FirstRun::ToolsBrew
            } else {
                FirstRun::Tools
            };
        }
        if self.agents.has_installed() {
            FirstRun::ReadyAgents
        } else {
            FirstRun::Ready
        }
    }

    fn tools_resolved(&self) -> bool {
        self.flow.tools_done || matches!(self.command, Some(CommandState::Installed { .. }))
    }

    /// A `reoclo` the app does not own answers on this Mac, so the app leaves it alone.
    fn another_command_in_charge(&self) -> bool {
        matches!(self.command, Some(CommandState::AlreadyPresent { .. }))
            || matches!(
                self.cli,
                Some(Cli::Ready { bundled: false, .. } | Cli::TooOld { .. })
            )
    }
}
