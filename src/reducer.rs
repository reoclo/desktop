pub use crate::logs::LogEntry;
use crate::model::{
    Approval, AuthEvent, ForegroundList, ForegroundTunnel, HostEntry, Status, TunnelTraffic,
    TunnelView,
};
use serde::Deserialize;
use std::collections::VecDeque;

#[derive(Clone, Debug, Default)]
pub struct Model {
    pub history: crate::traffic::History,
    pub status: Status,
    pub tunnels: Vec<TunnelView>,
    pub foreground: Vec<ForegroundTunnel>,
    pub approvals: Vec<Approval>,
    pub host_entries: Vec<HostEntry>,
    pub auth: AuthEvent,
    pub selected: Option<String>,
    pub recent_logs: VecDeque<LogEntry>,
    pub log_error: Option<String>,
    pub next_log_id: u64,
}

impl Model {
    pub fn apply(&mut self, event: &str, data: &str) -> Result<(), serde_json::Error> {
        self.apply_at(event, data, time::OffsetDateTime::now_utc())
    }

    pub fn apply_at(
        &mut self,
        event: &str,
        data: &str,
        at: time::OffsetDateTime,
    ) -> Result<(), serde_json::Error> {
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
                    self.history.remove(&change.name);
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
                        self.history
                            .record(&next.name, &next.traffic, at.unix_timestamp());
                        tunnel.traffic = next.traffic;
                    } else if let Some(id) = next.name.strip_prefix("terminal:")
                        && let Some(tunnel) = self.foreground.iter_mut().find(|t| t.id == id)
                    {
                        self.history
                            .record(&next.name, &next.traffic, at.unix_timestamp());
                        tunnel.traffic = Some(next.traffic);
                    }
                }
                self.history.prune(at.unix_timestamp());
            }
            "foreground" => {
                let tunnels = serde_json::from_str::<ForegroundList>(data)?.tunnels;
                for old in &self.foreground {
                    if !tunnels.iter().any(|t| t.id == old.id) {
                        self.history.remove(&format!("terminal:{}", old.id));
                    }
                }
                self.foreground = tunnels;
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
                self.append_log(LogEntry::event(serde_json::from_str::<Log>(data)?.line, at));
            }
            _ => {}
        }
        Ok(())
    }

    pub fn append_log(&mut self, mut entry: LogEntry) {
        // SSE and the file carry the same daemon message. Pair each copy once, keeping the
        // original disk timestamp. Repeated messages from distinct events still get rows.
        if let Some(other) = self.recent_logs.iter_mut().rev().take(200).find(|other| {
            !other.paired
                && other.persisted != entry.persisted
                && other.line == entry.line
                && (other.received - entry.received).whole_seconds().abs() <= 2
        }) {
            if entry.persisted {
                entry.id = other.id;
                *other = entry;
            }
            other.paired = true;
            return;
        }
        entry.id = self.next_log_id;
        self.next_log_id = self.next_log_id.wrapping_add(1);
        if self
            .recent_logs
            .back()
            .is_some_and(|last| last.received > entry.received)
        {
            let index = self
                .recent_logs
                .iter()
                .position(|old| old.received > entry.received)
                .unwrap_or(self.recent_logs.len());
            self.recent_logs.insert(index, entry);
        } else {
            self.recent_logs.push_back(entry);
        }
        while self.recent_logs.len() > crate::logs::MAX_LINES {
            self.recent_logs.pop_front();
        }
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
            } else {
                self.history.remove(&old.name);
            }
        }
        ordered.extend(tunnels);
        self.tunnels = ordered;
        if self.selected.as_ref().is_some_and(|name| {
            !name.starts_with("terminal:") && !self.tunnels.iter().any(|t| &t.name == name)
        }) {
            self.selected = None;
        }
    }
}
