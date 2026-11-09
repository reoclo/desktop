use crate::model::{Approval, AuthEvent, HostEntry, Status, TunnelTraffic, TunnelView};
use serde::Deserialize;
use std::collections::BTreeMap;

#[derive(Clone, Debug, Default)]
pub struct Model {
    pub status: Status,
    pub tunnels: Vec<TunnelView>,
    pub approvals: Vec<Approval>,
    pub host_entries: Vec<HostEntry>,
    pub auth: AuthEvent,
    pub selected: Option<String>,
    pub recent_logs: Vec<String>,
}

impl Model {
    pub fn apply(&mut self, event: &str, data: &str) -> Result<(), serde_json::Error> {
        match event {
            "tunnel" => {
                #[derive(Deserialize)]
                struct Change {
                    name: String,
                    #[serde(default)]
                    removed: bool,
                    tunnel: Option<TunnelView>,
                }
                let change: Change = serde_json::from_str(data)?;
                self.tunnels.retain(|t| t.name != change.name);
                if change.removed {
                    if self.selected.as_deref() == Some(&change.name) {
                        self.selected = None;
                    }
                } else if let Some(tunnel) = change.tunnel {
                    self.tunnels.push(tunnel);
                }
                self.sort();
            }
            "traffic" => {
                #[derive(Deserialize)]
                struct Traffic {
                    name: String,
                    traffic: TunnelTraffic,
                }
                #[derive(Deserialize)]
                struct TrafficBatch {
                    tunnels: Vec<Traffic>,
                }
                for next in serde_json::from_str::<TrafficBatch>(data)?.tunnels {
                    if let Some(tunnel) = self.tunnels.iter_mut().find(|t| t.name == next.name) {
                        tunnel.traffic = next.traffic;
                    }
                }
            }
            "auth" => self.auth = serde_json::from_str(data)?,
            "approvals" | "approval" => {
                #[derive(Deserialize)]
                struct Approvals {
                    approvals: Vec<Approval>,
                }
                self.approvals = serde_json::from_str::<Approvals>(data)?.approvals;
            }
            "log" => {
                #[derive(Deserialize)]
                struct Log {
                    line: String,
                }
                self.recent_logs
                    .push(serde_json::from_str::<Log>(data)?.line);
                if self.recent_logs.len() > 50 {
                    self.recent_logs.remove(0);
                }
            }
            _ => {}
        }
        Ok(())
    }

    pub fn sort(&mut self) {
        self.tunnels
            .sort_by(|a, b| (&a.org, &a.server, &a.name).cmp(&(&b.org, &b.server, &b.name)));
    }

    pub fn groups(&self) -> BTreeMap<(&str, &str), Vec<&TunnelView>> {
        let mut groups = BTreeMap::new();
        for tunnel in &self.tunnels {
            groups
                .entry((tunnel.org.as_str(), tunnel.server.as_str()))
                .or_insert_with(Vec::new)
                .push(tunnel);
        }
        groups
    }
}
