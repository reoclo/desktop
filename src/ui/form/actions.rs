use crate::{control::segment, ui::State};
use serde_json::json;

pub(super) fn hostname(s: &State) -> String {
    let domain = if s.domain.is_empty() {
        "reoclo.localhost"
    } else {
        s.domain.as_str()
    };
    let typed = s.hostname.text.trim().to_ascii_lowercase();
    if !typed.is_empty() {
        // The domain itself, or a whole name already on it, is used as written.
        if typed == domain || typed.ends_with(&format!(".{domain}")) {
            return typed;
        }
        return format!("{typed}.{domain}");
    }
    let name = if s.name.text.trim().is_empty() {
        "tunnel"
    } else {
        s.name.text.trim()
    };
    if domain != "reoclo.localhost" {
        return format!("{name}.{domain}");
    }
    let server = s
        .servers
        .iter()
        .find(|server| server.id == s.server)
        .map(|server| server.slug.as_str())
        .unwrap_or("server");
    format!("{name}.{server}.reoclo.localhost")
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
    // The daemon stores the tenant id with each tunnel; the org list carries it as `id`.
    let Some(tenant_id) = s
        .orgs
        .iter()
        .find(|org| org.slug == s.org)
        .map(|org| org.id.clone())
    else {
        s.error = Some("Choose an organization and server.".into());
        return;
    };
    let mut body = json!({"name":name,"org":s.org,"server":server,"server_id":s.server,"kind":s.kind,"via":s.via});
    body["tenant_id"] = json!(tenant_id);
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
