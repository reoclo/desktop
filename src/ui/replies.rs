mod events;
use super::{Fault, PortsScan, Request, State, ViewState};
use crate::model::*;
use serde_json::Value;

impl State {
    pub fn receive(&mut self, request: &Request, response: Result<Value, Fault>) {
        self.apply_response(request, response);
        self.sync_tray();
    }

    fn apply_response(&mut self, request: &Request, response: Result<Value, Fault>) {
        self.pending.remove(&request.path);
        match response {
            Ok(value) => {
                if let Err(error) = self.accept(request, value) {
                    self.error = Some(format!("The daemon returned an invalid response: {error}"));
                    self.phase = ViewState::Error;
                    self.finish_trust_check(request, false);
                } else {
                    self.link_reply(request);
                    self.finish_trust_check(request, true);
                }
            }
            // Daemons from before the idle-timeout fix trust the CA, then drop the connection
            // without a reply, so ask the daemon what it did before reporting a failure.
            Err(Fault::Lost(message)) if request.path == "/v1/ca/trust" => {
                self.trust_check = Some((request.clone(), message));
                self.request("GET", "/v1/status", None);
            }
            // A daemon older than this feature has no such route, and nothing to report.
            Err(Fault::Error(_)) if request.path == "/v1/foreground" => {}
            // A scan that fails is reported beside the port picker, and the form stays usable.
            Err(Fault::Error(message) | Fault::Lost(message))
                if request.method == "GET" && request.path.ends_with("/ports") =>
            {
                self.ports_scan = PortsScan::Failed(message);
            }
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
                    Fault::Error(message) | Fault::Lost(message) => {
                        if request.path == "/v1/status"
                            || request.path == "/v1/tunnels" && request.method == "GET"
                        {
                            self.phase = ViewState::Error;
                        }
                        message
                    }
                });
                self.flow_failed(request);
                self.finish_trust_check(request, false);
            }
        }
    }

    /// A status answer settles a trust request whose reply was lost: done if the CA is now
    /// trusted, otherwise the original error. A status that failed leaves only the error.
    fn finish_trust_check(&mut self, request: &Request, status_ok: bool) {
        if request.path != "/v1/status" {
            return;
        }
        let Some((trust, message)) = self.trust_check.take() else {
            return;
        };
        let settled = if status_ok && self.ca_ready() {
            Ok(serde_json::json!({"ok": true}))
        } else {
            Err(Fault::Error(message))
        };
        self.apply_response(&trust, settled);
    }

    fn accept(&mut self, request: &Request, value: Value) -> Result<(), serde_json::Error> {
        match (request.method.as_str(), request.path.as_str()) {
            ("GET", "/v1/status") => {
                self.model.status = serde_json::from_value(value)?;
                self.refresh_service();
                self.remember_relay();
                self.find_cli(false);
                self.daemon_answered();
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
                self.remember_relay();
                if after_install {
                    self.secure_after_helper();
                    self.helper_answered();
                }
            }
            ("GET", "/v1/tunnels") => {
                self.model
                    .replace_tunnels(serde_json::from_value::<TunnelList>(value)?.tunnels);
                if self.model.status.signed_in == Some(false) {
                    self.phase = ViewState::SignedOut;
                } else {
                    self.phase = if self.model.tunnels.is_empty() {
                        ViewState::Empty
                    } else {
                        ViewState::Loaded
                    };
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
                match serde_json::from_value::<PortList>(value) {
                    Ok(list) => {
                        self.ports = list.ports;
                        self.ports_scan = PortsScan::Loaded;
                    }
                    Err(error) => {
                        self.ports_scan = PortsScan::Failed(format!(
                            "The daemon sent a port list the app cannot read: {error}"
                        ));
                    }
                }
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
                self.model.update_tunnel(tunnel);
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
                } else if !domain {
                    self.certificate_trusted();
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
