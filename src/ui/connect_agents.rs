//! Connect your agents: listing the coding agents on this Mac and registering Reoclo's MCP server with them.
use super::{FirstRun, State};
use crate::{
    agents::{self, Agent, Change},
    cli::Cli,
    system::{SystemAction, command_path::CommandState},
};
use std::path::PathBuf;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum AgentsStep {
    #[default]
    NotChecked,
    Checking,
    /// The reoclo command could not list the agents. The Ready screen goes on without them.
    Unavailable(String),
    Listed(Vec<Agent>),
    Connecting(Vec<Agent>),
    Done {
        agents: Vec<Agent>,
        results: Vec<Change>,
    },
    Failed {
        agents: Vec<Agent>,
        message: String,
    },
}

impl AgentsStep {
    pub fn found(&self) -> &[Agent] {
        match self {
            Self::NotChecked | Self::Checking | Self::Unavailable(_) => &[],
            Self::Listed(agents)
            | Self::Connecting(agents)
            | Self::Done { agents, .. }
            | Self::Failed { agents, .. } => agents,
        }
    }

    pub fn has_installed(&self) -> bool {
        self.found().iter().any(|agent| agent.installed)
    }
}

impl State {
    /// The reoclo command that lists and connects agents: the copy in the app, which has those commands.
    fn agents_program(&self) -> Option<PathBuf> {
        self.command_paths
            .as_ref()
            .map(|paths| paths.bundled.clone())
            .or_else(|| match &self.cli {
                Some(Cli::Ready { path, .. }) => Some(path.clone()),
                _ => None,
            })
    }

    /// What the agents run: the link on PATH when the app made it, the reoclo on PATH that stays in
    /// charge when another install answers to it, else the app's own copy.
    fn agent_command(&self, program: &std::path::Path) -> PathBuf {
        match &self.command {
            Some(CommandState::Installed { link }) => link.clone(),
            Some(CommandState::AlreadyPresent { path, .. }) => path.clone(),
            _ => program.to_owned(),
        }
    }

    /// Runs every frame. Lists the agents once, when the Ready screen is first reached.
    pub(super) fn first_run_tick(&mut self) -> bool {
        let ready = matches!(
            self.first_run_step(),
            FirstRun::Ready | FirstRun::ReadyAgents
        );
        if !(self.first_run && ready && self.agents == AgentsStep::NotChecked)
            || self.busy
            || self.system_runner.is_none()
        {
            return false;
        }
        match self.agents_program() {
            Some(program) => {
                self.agents = AgentsStep::Checking;
                self.run_system(SystemAction::ListAgents { program });
            }
            None => {
                self.agents = AgentsStep::Unavailable("No reoclo command was found to ask.".into());
            }
        }
        true
    }

    /// Connect your agents: one button registers Reoclo with every agent found.
    pub fn connect_agents(&mut self) {
        let (AgentsStep::Listed(found)
        | AgentsStep::Done { agents: found, .. }
        | AgentsStep::Failed { agents: found, .. }) = self.agents.clone()
        else {
            return;
        };
        let Some(program) = self.agents_program() else {
            return;
        };
        if self.busy {
            return;
        }
        let command = self.agent_command(&program);
        self.agents = AgentsStep::Connecting(found);
        self.run_system(SystemAction::ConnectAgents { program, command });
    }

    /// A list or connect action finished. Returns false for any other action.
    pub(super) fn agents_finished(
        &mut self,
        action: &SystemAction,
        result: &Result<String, String>,
    ) -> bool {
        let found = self.agents.found().to_vec();
        self.agents = match (action, result) {
            (SystemAction::ListAgents { .. }, Ok(json)) => match agents::parse_agents(json) {
                Ok(agents) => AgentsStep::Listed(agents),
                Err(error) => AgentsStep::Unavailable(error.to_string()),
            },
            (SystemAction::ListAgents { .. }, Err(message)) => {
                AgentsStep::Unavailable(message.clone())
            }
            (SystemAction::ConnectAgents { .. }, Ok(json)) => match agents::parse_changes(json) {
                Ok(results) => AgentsStep::Done {
                    agents: found,
                    results,
                },
                Err(error) => AgentsStep::Failed {
                    agents: found,
                    message: format!("Reoclo could not read the answer to Connect: {error}"),
                },
            },
            (SystemAction::ConnectAgents { .. }, Err(message)) => AgentsStep::Failed {
                agents: found,
                message: message.clone(),
            },
            _ => return false,
        };
        true
    }
}
