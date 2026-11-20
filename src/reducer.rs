use crate::model::{
    Approval, AuthEvent, ForegroundList, ForegroundTunnel, HostEntry, Status, TunnelTraffic,
    TunnelView,
};
use serde::Deserialize;
use std::collections::BTreeMap;

#[derive(Clone, Debug, Default)]
pub struct Model {
    pub status: Status,
    pub tunnels: Vec<TunnelView>,
    pub foreground: Vec<ForegroundTunnel>,
    pub approvals: Vec<Approval>,
    pub host_entries: Vec<HostEntry>,
    pub auth: AuthEvent,
    pub selected: Option<String>,
    pub recent_logs: Vec<LogEntry>,
}

#[derive(Clone, Debug)]
pub struct LogEntry {
    pub received: time::OffsetDateTime,
    pub line: String,
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
                if change.removed {
                    self.tunnels.retain(|t| t.name != change.name);
                    if self.selected.as_deref() == Some(&change.name) {
                        self.selected = None;
                    }
                } else if let Some(tunnel) = change.tunnel {
                    self.update_tunnel(tunnel);
                }
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
            "foreground" => self.foreground = serde_json::from_str::<ForegroundList>(data)?.tunnels,
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
                self.recent_logs.push(LogEntry {
                    received: time::OffsetDateTime::now_utc(),
                    line: serde_json::from_str::<Log>(data)?.line,
                });
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

    /// State changes preserve the user's row order.
    pub fn update_tunnel(&mut self, tunnel: TunnelView) {
        if let Some(current) = self.tunnels.iter_mut().find(|t| t.name == tunnel.name) {
            *current = tunnel;
        } else {
            self.tunnels.push(tunnel);
        }
    }

    pub fn replace_tunnels(&mut self, mut tunnels: Vec<TunnelView>) {
        let mut ordered = Vec::with_capacity(tunnels.len());
        for old in &self.tunnels {
            if let Some(index) = tunnels.iter().position(|t| t.name == old.name) {
                ordered.push(tunnels.remove(index));
            }
        }
        ordered.extend(tunnels);
        self.tunnels = ordered;
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
