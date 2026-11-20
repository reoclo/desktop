//! The coding agents on this Mac, as `reoclo mcp agents -o json` and `reoclo mcp install -o json` report them.
use serde::Deserialize;

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct Agent {
    pub id: String,
    pub name: String,
    pub installed: bool,
    pub config_path: String,
    /// None when the agent's config could not be read; `error` says why.
    pub registered: Option<bool>,
    pub command: Option<String>,
    pub error: Option<String>,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum Action {
    Installed,
    Updated,
    Removed,
    Unchanged,
    NotFound,
    Failed,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct Problem {
    pub kind: String,
    pub message: String,
}

/// What `reoclo mcp install` did for one agent.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct Change {
    pub agent: String,
    pub name: String,
    pub action: Action,
    pub config_path: String,
    pub error: Option<Problem>,
}

#[derive(Deserialize)]
struct Listed {
    agents: Vec<Agent>,
}

#[derive(Deserialize)]
struct Changed {
    results: Vec<Change>,
}

pub fn parse_agents(json: &str) -> Result<Vec<Agent>, serde_json::Error> {
    serde_json::from_str::<Listed>(json).map(|listed| listed.agents)
}

pub fn parse_changes(json: &str) -> Result<Vec<Change>, serde_json::Error> {
    serde_json::from_str::<Changed>(json).map(|changed| changed.results)
}
