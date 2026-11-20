//! First-run fixtures: `--screen first-run --state <step>` builds the plumbing state each screen reads.
use crate::{
    agents::{Action, Agent, Change, Problem},
    cli::Cli,
    link,
    model::{AuthEvent, Ca, Status},
    ui::{AgentsStep, LoginStep, PendingLink, State, StepState, ViewState},
};

const BUNDLED: &str = "/Applications/Reoclo.app/Contents/Resources/reoclo";

fn agent(id: &str, name: &str, path: &str) -> Agent {
    Agent {
        id: id.into(),
        name: name.into(),
        installed: true,
        config_path: path.into(),
        registered: Some(false),
        command: None,
        error: None,
    }
}

fn change(agent: &Agent, failure: Option<&str>) -> Change {
    Change {
        agent: agent.id.clone(),
        name: agent.name.clone(),
        action: if failure.is_some() {
            Action::Failed
        } else {
            Action::Installed
        },
        config_path: agent.config_path.clone(),
        error: failure.map(|message| Problem {
            kind: "parse".into(),
            message: message.into(),
        }),
    }
}

impl State {
    /// A Mac whose service runs, whose account is signed in and whose setup steps are all done.
    fn first_run_baseline(&mut self) {
        self.first_run = true;
        self.modal = None;
        self.phase = ViewState::Loaded;
        self.service_installed = true;
        self.model.status.signed_in = Some(true);
        self.model.status.ca = Some(Ca {
            exists: true,
            trusted: true,
            ..Ca::default()
        });
        if let Some(hosts) = &mut self.model.status.hosts {
            hosts.installed = true;
        }
        self.cli = Some(Cli::Ready {
            path: BUNDLED.into(),
            version: self.model.status.version.clone(),
            bundled: true,
        });
    }

    /// A Mac with only the app on it: the service is not running yet.
    fn first_run_fresh(&mut self) {
        self.first_run_baseline();
        self.phase = ViewState::Offline;
        self.model.status = Status::default();
    }

    fn first_run_unpaired(&mut self, certificate_trusted: bool) {
        self.first_run_baseline();
        if let Some(ca) = &mut self.model.status.ca {
            ca.trusted = certificate_trusted;
        }
        if let Some(hosts) = &mut self.model.status.hosts {
            hosts.installed = false;
        }
    }

    fn first_run_agents(&mut self, step: AgentsStep) {
        self.first_run_baseline();
        self.flow.tools_done = true;
        self.agents = step;
    }

    /// Sets up a first-run fixture. Unknown states leave the Mac set up, on the tools screen.
    pub(in crate::ui) fn first_run_fixture(&mut self, case: &str) {
        let found = [
            agent("claude-code", "Claude Code", "~/.claude.json"),
            agent("codex", "Codex", "~/.codex/config.toml"),
        ];
        match case {
            "welcome" => self.first_run_fresh(),
            "welcome-failed" => {
                self.first_run_fresh();
                self.flow.login = LoginStep::Failed(
                    "Reoclo's background service did not start in time. Try again.".into(),
                );
            }
            "link" => {
                self.first_run_fresh();
                self.link = link::parse(
                    "reoclo://open?v=1&org=flowstate&server=generator-queue-01&port=14400&kind=https&name=status",
                )
                .ok()
                .map(PendingLink::new);
            }
            "installing" => {
                self.first_run_fresh();
                self.flow.login = LoginStep::InstallingService;
            }
            "waiting" => {
                self.first_run_baseline();
                self.model.status.signed_in = Some(false);
                self.phase = ViewState::SignedOut;
                self.flow.login = LoginStep::WaitingForApproval;
                self.model.auth = AuthEvent {
                    user_code: Some("KQTM-7294".into()),
                    verification_uri: Some("https://auth.reoclo.com/device".into()),
                    status: "pending".into(),
                    ..Default::default()
                };
            }
            "pairing" => self.first_run_unpaired(false),
            "pairing-failed" => {
                self.first_run_unpaired(false);
                self.flow.certificate = StepState::Failed("The trust change was canceled.".into());
            }
            "pairing-helper" => self.first_run_unpaired(true),
            "tools-homebrew" => {
                self.first_run_baseline();
                self.cli = Some(Cli::Ready {
                    path: "/opt/homebrew/bin/reoclo".into(),
                    version: self.model.status.version.clone(),
                    bundled: false,
                });
            }
            "tools-failed" => {
                self.first_run_baseline();
                self.flow.tools = StepState::Failed(
                    "Reoclo could not write to your shell settings. Check that ~/.zshenv is not locked.".into(),
                );
            }
            "ready" => self.first_run_agents(AgentsStep::Unavailable(String::new())),
            "ready-agents" => self.first_run_agents(AgentsStep::Listed(found.to_vec())),
            "ready-connected" => {
                let results = vec![
                    change(&found[0], None),
                    change(&found[1], Some("config.toml is not valid TOML")),
                ];
                self.first_run_agents(AgentsStep::Done {
                    agents: found.to_vec(),
                    results,
                });
            }
            "loading" => {
                self.first_run_baseline();
                self.phase = ViewState::Loading;
            }
            _ => self.first_run_baseline(),
        }
    }
}
