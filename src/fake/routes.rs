use super::Fixture;
use crate::model::{Settings, TunnelState, TunnelView};
use serde_json::{Value, json};

type Reply = (u16, Value, Option<String>);

fn event(name: &str, body: &Value) -> Option<String> {
    Some(format!("event: {name}\ndata: {body}\n\n"))
}

fn fail(code: u16, message: &str) -> Reply {
    (
        code,
        json!({"error":{"code":if code == 404 {"not_found"} else {"invalid"},"message":message}}),
        None,
    )
}

pub(super) fn route(f: &mut Fixture, method: &str, path: &str, input: Value) -> Reply {
    match (method, path) {
        ("GET", "/v1/status") => return (200, json!(f.status), None),
        ("GET", "/v1/hosts") => {
            return (
                200,
                json!({"entries": f.host_entries, "helper": f.status.hosts}),
                None,
            );
        }
        ("GET", "/v1/tunnels") => return (200, json!({"tunnels":f.tunnels}), None),
        ("GET", "/v1/orgs") => return (200, json!({"orgs":f.orgs}), None),
        ("GET", "/v1/approvals") => return (200, json!({"approvals":f.approvals}), None),
        ("GET", "/v1/approvals?all=true") => {
            let all: Vec<_> = f
                .approvals
                .iter()
                .chain(f.approval_history.iter())
                .collect();
            return (200, json!({"approvals":all}), None);
        }
        ("GET", "/v1/settings") => return (200, json!(f.settings), None),
        ("PATCH", "/v1/settings") => {
            let mut value = json!(f.settings);
            if let (Some(dst), Some(src)) = (value.as_object_mut(), input.as_object()) {
                dst.extend(src.clone());
            }
            let Ok(settings) = serde_json::from_value::<Settings>(value) else {
                return fail(400, "Invalid settings");
            };
            if settings.https_port == 0
                || settings.idle_timeout_seconds == 0
                || !matches!(settings.agent_approvals.as_str(), "required" | "off")
            {
                return fail(
                    400,
                    "Use a port from 1 to 65535, a positive idle timeout, and valid approval mode",
                );
            }
            f.settings = settings;
            if let Some(https) = &mut f.status.https {
                https.port = f.settings.https_port;
            }
            return (200, json!(f.settings), None);
        }
        ("POST", "/v1/auth/login") => {
            let body = json!({"status":"pending","user_code":"DEMO-7294","verification_uri":"https://auth.reoclo.com/device"});
            return (202, json!({"ok":true}), event("auth", &body));
        }
        ("POST", "/v1/ca/trust" | "/v1/ca/untrust") => {
            if let Some(ca) = &mut f.status.ca {
                ca.trusted = path.ends_with("/trust");
            }
            return (200, json!({"ok":true}), None);
        }
        ("POST", "/v1/tunnels") => {
            let Ok(mut tunnel) = serde_json::from_value::<TunnelView>(input) else {
                return fail(400, "Invalid tunnel");
            };
            if tunnel.name.is_empty() || tunnel.server_id.is_empty() {
                return fail(400, "Name and server are required");
            }
            if f.tunnels.iter().any(|t| t.name == tunnel.name) {
                return fail(409, "A tunnel already uses that name");
            }
            tunnel.enabled = true;
            tunnel.state = TunnelState::Ready;
            tunnel.message = "Listening, no session open".into();
            if tunnel.kind == "https" {
                tunnel.url = tunnel
                    .hostname
                    .as_ref()
                    .map(|h| format!("https://{h}:{}", f.settings.https_port));
            } else {
                tunnel.endpoint = Some(format!("127.0.0.1:{}", tunnel.local_port.unwrap_or(15432)));
            }
            f.tunnels.push(tunnel.clone());
            let changed = json!({"type":"tunnel","name":tunnel.name,"tunnel":tunnel});
            return (201, json!(tunnel), event("tunnel", &changed));
        }
        _ => {}
    }
    let parts: Vec<_> = path.trim_start_matches('/').split('/').collect();
    if method == "GET" && parts.get(1) == Some(&"orgs") {
        if parts.last() == Some(&"ports") {
            return (200, json!({"ports":f.ports}), None);
        }
        if parts.last() == Some(&"servers") {
            return (200, json!({"servers":f.servers}), None);
        }
    }
    if method == "POST" && parts.get(1) == Some(&"approvals") {
        let id = parts.get(2).copied().unwrap_or("");
        if !matches!(parts.get(3), Some(&"approve" | &"deny")) {
            return fail(404, path);
        }
        let Some(index) = f.approvals.iter().position(|a| a.id == id) else {
            return fail(404, "Approval not found");
        };
        let mut approval = f.approvals.remove(index);
        approval.status = if parts.get(3) == Some(&"deny") {
            "denied"
        } else {
            "approved"
        }
        .into();
        f.approval_history.push(approval.clone());
        return (
            200,
            json!(approval),
            event(
                "approval",
                &json!({"approvals":f.approvals,"approval":approval}),
            ),
        );
    }
    if parts.get(1) == Some(&"tunnels") {
        let name = parts.get(2).copied().unwrap_or("");
        let Some(index) = f.tunnels.iter().position(|t| t.name == name) else {
            return fail(404, &format!("Tunnel {name} does not exist"));
        };
        if method == "DELETE" {
            f.tunnels.remove(index);
            return (
                200,
                json!({"ok":true}),
                event(
                    "tunnel",
                    &json!({"type":"tunnel","name":name,"removed":true}),
                ),
            );
        }
        let tunnel = &mut f.tunnels[index];
        if method == "GET" {
            return (200, json!(tunnel), None);
        }
        if method == "PATCH" {
            let mut value = json!(tunnel);
            if let (Some(dst), Some(src)) = (value.as_object_mut(), input.as_object()) {
                dst.extend(src.clone());
            }
            let Ok(updated) = serde_json::from_value(value) else {
                return fail(400, "Invalid tunnel update");
            };
            *tunnel = updated;
        } else if method == "POST" && matches!(parts.get(3), Some(&"up" | &"down")) {
            tunnel.enabled = parts.get(3) == Some(&"up");
            tunnel.state = if tunnel.enabled {
                TunnelState::Ready
            } else {
                TunnelState::Down
            };
            tunnel.message = if tunnel.enabled {
                "Listening, no session open"
            } else {
                "Tunnel is down"
            }
            .into();
        } else {
            return fail(404, path);
        }
        return (
            200,
            json!(tunnel),
            event(
                "tunnel",
                &json!({"type":"tunnel","name":tunnel.name,"tunnel":tunnel}),
            ),
        );
    }
    fail(404, &format!("No route for {method} {path}"))
}
