//! The `reoclo://open` resource link, shared with the web dashboard.
//!
//! `reoclo://open?v=1&org=<org slug>&server=<server slug or id>&port=<1-65535>&kind=<https|tcp>`
//! with the optional `host=<host on the server>` (default 127.0.0.1) and `name=<tunnel name>`.
//! Values are URL-encoded and unknown parameters are ignored. Links can come from anyone, so
//! every value is checked before anything acts on it.

use std::net::{Ipv4Addr, Ipv6Addr};

/// How the resource is reached from this Mac.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LinkKind {
    Https,
    Tcp,
}

impl LinkKind {
    /// The wire value the daemon uses for the tunnel kind.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Https => "https",
            Self::Tcp => "tcp",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResourceLink {
    pub org: String,
    pub server: String,
    pub port: u16,
    pub kind: LinkKind,
    pub host: String,
    pub name: String,
}

impl ResourceLink {
    /// The tunnel target on the server, as `host:port`. An IPv6 host is bracketed.
    pub fn target(&self) -> String {
        if self.host.contains(':') {
            format!("[{}]:{}", self.host, self.port)
        } else {
            format!("{}:{}", self.host, self.port)
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum LinkError {
    #[error("This is not a Reoclo resource link.")]
    NotALink,
    #[error("This link was made for a newer version of Reoclo. Update the app and try again.")]
    Version,
    #[error("This link is missing the {0}.")]
    Missing(&'static str),
    #[error("This link has an invalid {0}.")]
    Invalid(&'static str),
    #[error("This link repeats the {0}.")]
    Repeated(&'static str),
}

const PREFIX: &str = "reoclo://open";

fn after_prefix(url: &str) -> Option<&str> {
    let head = url.get(..PREFIX.len())?;
    head.eq_ignore_ascii_case(PREFIX)
        .then(|| &url[PREFIX.len()..])
}

/// True when the URL is addressed to the resource link handler, valid or not.
pub fn is_open_link(url: &str) -> bool {
    after_prefix(url).is_some_and(|rest| rest.is_empty() || rest.starts_with(['?', '/', '#']))
}

fn decode(value: &str) -> Option<String> {
    let bytes = value.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'%' => {
                let pair = value.get(i + 1..i + 3)?;
                out.push(u8::from_str_radix(pair, 16).ok()?);
                i += 3;
            }
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            byte => {
                out.push(byte);
                i += 1;
            }
        }
    }
    String::from_utf8(out).ok()
}

/// The tunnel name rule: `^[a-z0-9][a-z0-9-]{0,62}$`. Organization slugs follow it too.
pub fn valid_name(value: &str) -> bool {
    let bytes = value.as_bytes();
    (1..=63).contains(&bytes.len())
        && (bytes[0].is_ascii_lowercase() || bytes[0].is_ascii_digit())
        && bytes[1..]
            .iter()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || *b == b'-')
}

/// A server slug, or the id the API gave the server.
fn valid_server(value: &str) -> bool {
    let bytes = value.as_bytes();
    (1..=64).contains(&bytes.len())
        && bytes[0].is_ascii_alphanumeric()
        && bytes[1..]
            .iter()
            .all(|b| b.is_ascii_alphanumeric() || *b == b'-' || *b == b'_')
}

fn valid_hostname(value: &str) -> bool {
    value.len() <= 253
        && value.split('.').all(|label| {
            let bytes = label.as_bytes();
            (1..=63).contains(&bytes.len())
                && bytes[0] != b'-'
                && bytes[bytes.len() - 1] != b'-'
                && bytes
                    .iter()
                    .all(|b| b.is_ascii_alphanumeric() || *b == b'-')
        })
}

/// A hostname or an IP address. An IPv6 address may be bracketed; the brackets are dropped.
fn host(value: &str) -> Option<String> {
    let value = value.to_ascii_lowercase();
    let bare = value
        .strip_prefix('[')
        .and_then(|rest| rest.strip_suffix(']'))
        .unwrap_or(&value);
    if bare.parse::<Ipv6Addr>().is_ok() {
        return Some(bare.to_owned());
    }
    (value.parse::<Ipv4Addr>().is_ok() || valid_hostname(&value)).then_some(value)
}

fn port(value: &str) -> Option<u16> {
    if value.is_empty() || !value.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    value.parse::<u16>().ok().filter(|port| *port != 0)
}

/// `<server>-<port>`, bent to fit the name rule.
fn derived_name(server: &str, port: u16) -> String {
    let suffix = format!("-{port}");
    let mut base: String = server
        .to_ascii_lowercase()
        .chars()
        .map(|c| {
            if c.is_ascii_lowercase() || c.is_ascii_digit() {
                c
            } else {
                '-'
            }
        })
        .collect();
    base.truncate(63 - suffix.len());
    let base = base.trim_matches('-');
    if base.is_empty() {
        format!("server{suffix}")
    } else {
        format!("{base}{suffix}")
    }
}

pub fn parse(url: &str) -> Result<ResourceLink, LinkError> {
    if !is_open_link(url) {
        return Err(LinkError::NotALink);
    }
    let rest = after_prefix(url).unwrap_or_default();
    let rest = rest.strip_prefix('/').unwrap_or(rest);
    let query = rest.strip_prefix('?').unwrap_or("");
    let query = query.split('#').next().unwrap_or_default();

    let mut version = None;
    let mut org = None;
    let mut server = None;
    let mut port_value = None;
    let mut kind = None;
    let mut host_value = None;
    let mut name = None;
    for pair in query.split('&').filter(|pair| !pair.is_empty()) {
        let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
        let (slot, label) = match decode(key).as_deref() {
            Some("v") => (&mut version, "version"),
            Some("org") => (&mut org, "organization"),
            Some("server") => (&mut server, "server"),
            Some("port") => (&mut port_value, "port"),
            Some("kind") => (&mut kind, "kind"),
            Some("host") => (&mut host_value, "host"),
            Some("name") => (&mut name, "name"),
            _ => continue,
        };
        let Some(value) = decode(value) else {
            return Err(LinkError::Invalid("encoding"));
        };
        if slot.replace(value).is_some() {
            return Err(LinkError::Repeated(label));
        }
    }

    match version.as_deref() {
        None => return Err(LinkError::Missing("version")),
        Some("1") => {}
        Some(_) => return Err(LinkError::Version),
    }
    let org = org
        .filter(|value| !value.is_empty())
        .ok_or(LinkError::Missing("organization"))?;
    if !valid_name(&org) {
        return Err(LinkError::Invalid("organization"));
    }
    let server = server
        .filter(|value| !value.is_empty())
        .ok_or(LinkError::Missing("server"))?;
    if !valid_server(&server) {
        return Err(LinkError::Invalid("server"));
    }
    let port = port_value
        .filter(|value| !value.is_empty())
        .ok_or(LinkError::Missing("port"))?;
    let port = self::port(&port).ok_or(LinkError::Invalid("port"))?;
    let kind = match kind.as_deref() {
        None | Some("") => return Err(LinkError::Missing("kind")),
        Some("https") => LinkKind::Https,
        Some("tcp") => LinkKind::Tcp,
        Some(_) => return Err(LinkError::Invalid("kind")),
    };
    let host = match host_value.filter(|value| !value.is_empty()) {
        None => "127.0.0.1".to_owned(),
        Some(value) => self::host(&value).ok_or(LinkError::Invalid("host"))?,
    };
    let name = match name.filter(|value| !value.is_empty()) {
        None => derived_name(&server, port),
        Some(value) if valid_name(&value) => value,
        Some(_) => return Err(LinkError::Invalid("name")),
    };
    Ok(ResourceLink {
        org,
        server,
        port,
        kind,
        host,
        name,
    })
}
