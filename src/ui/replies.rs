use super::{Fault, Request, State, ViewState};
use crate::model::*;
use serde_json::Value;

impl State {
    pub fn receive(&mut self, request: &Request, response: Result<Value, Fault>) {
        self.busy = false;
        match response {
            Ok(value) => {
                if let Err(error) = self.accept(request, value) {
                    self.error = Some(format!("The daemon returned an invalid response: {error}"));
                    self.phase = ViewState::Error;
                }
            }
            Err(fault) => {
                self.error = Some(match fault {
                    Fault::Offline(message) => {
                        self.phase = ViewState::Offline;
                        message
                    }
                    Fault::SignedOut(message) => {
                        self.phase = ViewState::SignedOut;
                        message
                    }
                    Fault::Error(message) => {
                        if request.path == "/v1/status"
                            || request.path == "/v1/tunnels" && request.method == "GET"
                        {
                            self.phase = ViewState::Error;
                        }
                        message
                    }
                });
            }
        }
    }

    fn accept(&mut self, request: &Request, value: Value) -> Result<(), serde_json::Error> {
        match (request.method.as_str(), request.path.as_str()) {
            ("GET", "/v1/status") => self.model.status = serde_json::from_value(value)?,
            ("GET", "/v1/hosts") => {
                let hosts: HostsList = serde_json::from_value(value)?;
                self.model.host_entries = hosts.entries;
                self.model.status.hosts = Some(hosts.helper);
            }
            ("GET", "/v1/tunnels") => {
                self.model.tunnels = serde_json::from_value::<TunnelList>(value)?.tunnels;
                self.model.sort();
                if self.model.status.signed_in == Some(false) {
                    self.phase = ViewState::SignedOut;
                } else {
                    self.phase = if self.model.tunnels.is_empty() {
                        ViewState::Empty
                    } else {
                        ViewState::Loaded
                    };
                }
                if self.selected().is_none() {
                    self.model.selected = self.model.tunnels.first().map(|t| t.name.clone());
                    if let Some(name) = &self.model.selected {
                        self.table.select(name);
                    }
                }
            }
            ("GET", "/v1/orgs") => {
                self.orgs = serde_json::from_value::<OrgList>(value)?.orgs;
                if self.org.is_empty() {
                    self.org = self
                        .orgs
                        .first()
                        .map(|o| o.slug.clone())
                        .unwrap_or_default();
                }
            }
            ("GET", "/v1/approvals") => {
                self.model.approvals = serde_json::from_value::<ApprovalList>(value)?.approvals
            }
            (_, "/v1/settings") => {
                self.settings = serde_json::from_value(value)?;
                self.idle = rune::TextEdit::new(self.settings.idle_timeout_seconds.to_string());
                self.https_port = rune::TextEdit::new(self.settings.https_port.to_string());
                self.approvals_required = self.settings.agent_approvals == "required";
                if request.method == "PATCH" {
                    self.notice = Some("Daemon settings saved.".into());
                }
            }
            ("POST", "/v1/auth/login") => {
                self.notice = Some("Waiting for a device code from the daemon.".into())
            }
            ("POST", "/v1/tunnels") => {
                let tunnel: TunnelView = serde_json::from_value(value)?;
                self.model.selected = Some(tunnel.name.clone());
                self.model.tunnels.push(tunnel);
                self.model.sort();
                self.modal = None;
                self.phase = ViewState::Loaded;
            }
            ("GET", path) if path.ends_with("/servers") => {
                self.servers = serde_json::from_value::<ServerList>(value)?.servers;
            }
            ("GET", path) if path.ends_with("/ports") => {
                self.ports = serde_json::from_value::<PortList>(value)?.ports
            }
            ("POST", path) if path.starts_with("/v1/approvals/") => {
                let approval: Approval = serde_json::from_value(value)?;
                self.model.approvals.retain(|a| a.id != approval.id);
                if let Some(error) = approval.error {
                    self.error = Some(format!(
                        "The approved action failed: {} ({})",
                        error.message, error.code
                    ));
                } else {
                    self.notice = Some(
                        if approval.status == "denied" {
                            "Request denied."
                        } else {
                            "Request approved."
                        }
                        .into(),
                    );
                }
            }
            ("DELETE", _) => {
                self.modal = None;
                self.model.selected = None;
            }
            ("PATCH" | "POST", path) if path.starts_with("/v1/tunnels/") => {
                let tunnel: TunnelView = serde_json::from_value(value)?;
                self.model.tunnels.retain(|t| t.name != tunnel.name);
                self.model.tunnels.push(tunnel);
                self.model.sort();
                self.modal = None;
            }
            _ => {
                self.notice = Some("Change applied.".into());
                self.modal = None;
            }
        }
        Ok(())
    }
}
