use crate::{control::segment, ui::State};
use serde_json::json;

pub(super) fn hostname(s: &State) -> String {
    if !s.hostname.text.trim().is_empty() {
        return s.hostname.text.trim().into();
    }
    let server = s
        .servers
        .iter()
        .find(|server| server.id == s.server)
        .map(|server| server.slug.as_str())
        .unwrap_or("server");
    format!(
        "{}.{}.reoclo.localhost",
        if s.name.text.is_empty() {
            "tunnel"
        } else {
            s.name.text.trim()
        },
        server
    )
}

pub(super) fn save(s: &mut State) {
    let name = s.name.text.trim().to_owned();
    if name.is_empty()
        || !name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"-_".contains(&b))
    {
        s.error = Some("Use a tunnel name with letters, numbers, hyphens or underscores.".into());
        return;
    }
    if s.org.is_empty() || s.server.is_empty() {
        s.error = Some("Choose an organization and server.".into());
        return;
    }
    if s.target
        .text
        .rsplit_once(':')
        .and_then(|(_, p)| p.parse::<u16>().ok())
        .is_none()
    {
        s.error = Some("Enter a target as host:port.".into());
        return;
    }
    let server = s
        .servers
        .iter()
        .find(|server| server.id == s.server)
        .map(|server| server.slug.clone())
        .unwrap_or_default();
    let mut body = json!({"name":name,"org":s.org,"server":server,"server_id":s.server,"kind":s.kind,"via":s.via});
    if s.kind == "reverse" {
        body["local_target"] = json!(s.target.text.trim());
    } else {
        body["target"] = json!(s.target.text.trim());
    }
    if s.kind == "https" {
        body["hostname"] = json!(hostname(s));
    } else {
        let Ok(port) = s.local_port.text.parse::<u16>() else {
            s.error = Some("Enter a port from 1 to 65535.".into());
            return;
        };
        if port == 0 {
            s.error = Some("Port must be greater than zero.".into());
            return;
        }
        body[if s.kind == "reverse" {
            "remote_port"
        } else {
            "local_port"
        }] = json!(port);
    }
    if s.modal.as_deref() == Some("edit") {
        s.request(
            "PATCH",
            &format!("/v1/tunnels/{}", segment(&name)),
            Some(body),
        );
    } else {
        s.request("POST", "/v1/tunnels", Some(body));
    }
}
