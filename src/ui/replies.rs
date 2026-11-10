use super::{Fault, Request, State, ViewState};
use crate::model::*;
use serde_json::Value;

impl State {
    pub fn receive(&mut self, request: &Request, response: Result<Value, Fault>) {
        self.apply_response(request, response);
        self.sync_tray();
    }

    /// A daemon event arrived on the event stream.
    pub fn apply_event(&mut self, name: &str, data: &str) {
        if let Err(error) = self.model.apply(name, data) {
            self.error = Some(format!("Invalid daemon event: {error}"));
        }
        match name {
            "auth" => match self.model.auth.status.as_str() {
                "success" => {
                    self.phase = ViewState::Loaded;
                    self.error = None;
                    self.model.status.signed_in = Some(true);
                    self.link_signed_in();
                }
                "pending" => self.phase = ViewState::SignedOut,
                "failed" => {
                    self.phase = ViewState::SignedOut;
                    self.error = self.model.auth.message.clone();
                    self.link = None;
                }
                _ => {}
            },
            "tunnel" => self.link_check_tunnel(),
            _ => {}
        }
        self.sync_tray();
    }

    fn apply_response(&mut self, request: &Request, response: Result<Value, Fault>) {
        self.busy = false;
        match response {
            Ok(value) => {
                if let Err(error) = self.accept(request, value) {
                    self.error = Some(format!("The daemon returned an invalid response: {error}"));
                    self.phase = ViewState::Error;
                } else {
                    self.link_reply(request);
                }
            }
            // A daemon older than this feature has no such route, and nothing to report.
            Err(Fault::Error(_)) if request.path == "/v1/foreground" => {}
            Err(fault) => {
                self.secure_failed(request);
                self.link_failed(request);
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
            ("GET", "/v1/status") => {
                self.model.status = serde_json::from_value(value)?;
                self.refresh_service();
                self.find_cli(false);
            }
            ("GET", "/v1/foreground") => {
                self.model.foreground = serde_json::from_value::<ForegroundList>(value)?.tunnels;
            }
            ("POST", path) if path.starts_with("/v1/foreground/") => {
                let id = path.split('/').nth(3).unwrap_or_default();
                self.model.foreground.retain(|t| t.id != id);
                self.modal = None;
                self.stop_target = None;
                self.notice = Some("Tunnel stopped.".into());
            }
            ("GET", "/v1/hosts") | ("POST", "/v1/hosts/sync") => {
                let hosts: HostsList = serde_json::from_value(value)?;
                let after_install = std::mem::take(&mut self.awaiting_helper);
                if after_install {
                    if hosts.helper.installed {
                        self.notice = Some("Hosts helper installed.".into());
                    } else {
                        self.error = Some(
                            "macOS finished the install, but the helper is not answering. See /var/log/reoclo-hostsd.log."
                                .into(),
                        );
                    }
                }
                self.model.host_entries = hosts.entries;
                self.model.status.hosts = Some(hosts.helper);
                if after_install {
                    self.secure_after_helper();
                }
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
                if request.method == "PATCH"
                    && request
                        .body
                        .as_ref()
                        .is_some_and(|b| b.get("ca_suffixes").is_some())
                {
                    // The CA's constraint is fixed when it is issued, so trust issues one without the removed suffix.
                    if self.modal.as_deref() == Some("remove-suffix") {
                        self.modal = None;
                    }
                    self.reissuing = true;
                    self.request("POST", "/v1/ca/trust", None);
                    self.notice = Some(
                        "Domain removed. Approve the password prompt to trust the new CA.".into(),
                    );
                } else if request.method == "PATCH" {
                    let hosts = request
                        .body
                        .as_ref()
                        .is_some_and(|b| b.get("manual_hosts").is_some());
                    if hosts {
                        self.new_host = rune::TextEdit::new("");
                    }
                    self.notice = Some(
                        if hosts {
                            "Hostnames updated."
                        } else {
                            "Daemon settings saved."
                        }
                        .into(),
                    );
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
                if let Some(wanted) = self.pending_server.take()
                    && let Some(found) = self
                        .servers
                        .iter()
                        .find(|o| o.id == wanted || o.slug == wanted || o.name == wanted)
                {
                    self.server = found.id.clone();
                }
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
            ("POST", "/v1/ca/trust") => {
                let domain = request
                    .body
                    .as_ref()
                    .is_some_and(|b| b.get("domains").is_some());
                self.notice = Some(
                    if std::mem::take(&mut self.reissuing) {
                        "Domain removed and the new CA trusted."
                    } else if domain {
                        "Domain trusted."
                    } else {
                        "Local CA trusted."
                    }
                    .into(),
                );
                self.trust_domain = rune::TextEdit::new("");
                // Trusting from the Add tunnel form keeps the form open so it can be saved.
                if self.modal.as_deref() == Some("trust") {
                    self.modal = None;
                }
                if self.securing && !domain {
                    self.secure_after_trust();
                }
            }
            _ => {
                self.notice = Some("Change applied.".into());
                self.modal = None;
            }
        }
        Ok(())
    }
}
