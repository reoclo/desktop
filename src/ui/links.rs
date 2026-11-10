use super::{Page, Request, State, ViewState, form::actions::default_hostname};
use crate::{
    control::segment,
    link::{self, LinkKind, ResourceLink},
    model::TunnelState,
};
use serde_json::json;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LinkStep {
    Confirm,
    SignIn,
    Orgs,
    Servers,
    Secure,
    Reaching,
}

/// A resource link past parsing, and how far along it is.
#[derive(Clone, Debug)]
pub struct PendingLink {
    pub link: ResourceLink,
    pub step: LinkStep,
    pub tenant_id: String,
    pub server_id: String,
    pub server_slug: String,
    /// The tunnel the link reaches, once it is known.
    pub tunnel: String,
    /// The clock time the current step gives up, set the first time a frame sees it.
    pub deadline: Option<f64>,
}

impl PendingLink {
    pub fn new(link: ResourceLink) -> Self {
        Self {
            link,
            step: LinkStep::Confirm,
            tenant_id: String::new(),
            server_id: String::new(),
            server_slug: String::new(),
            tunnel: String::new(),
            deadline: None,
        }
    }
}

/// Work a reply decided on that needs the window.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Effect {
    OpenUrl(String),
    Copy(String),
}

const REACH_SECONDS: f64 = 60.0;
const SIGN_IN_SECONDS: f64 = 900.0;

fn failure(state: TunnelState) -> bool {
    matches!(
        state,
        TunnelState::Failed
            | TunnelState::Forbidden
            | TunnelState::Expired
            | TunnelState::ClosedByOperator
            | TunnelState::QuotaReached
            | TunnelState::ServerOffline
            | TunnelState::NeedsSignIn
    )
}

impl State {
    pub fn link_step(&self) -> Option<LinkStep> {
        self.link.as_ref().map(|pending| pending.step)
    }

    /// A `reoclo://open` link arrived. Nothing happens until the person confirms it.
    pub fn open_link(&mut self, url: &str) {
        self.page = Page::Tunnels;
        match link::parse(url) {
            Ok(link) => {
                self.error = None;
                self.close_pickers();
                self.link = Some(PendingLink::new(link));
                self.modal = Some("open-link".into());
            }
            Err(error) => self.error = Some(error.to_string()),
        }
    }

    /// What the confirmation dialog says the link will do.
    pub fn link_description(&self) -> Option<String> {
        let pending = self
            .link
            .as_ref()
            .filter(|pending| pending.step == LinkStep::Confirm)?;
        let link = &pending.link;
        let (what, after) = match link.kind {
            LinkKind::Https => (
                format!("a secure web address (HTTPS) for {}", link.target()),
                "It then opens the address in your browser.",
            ),
            LinkKind::Tcp => (
                format!(
                    "a local port on this Mac that forwards to {}",
                    link.target()
                ),
                "It then copies the local address to your clipboard.",
            ),
        };
        Some(format!(
            "This link wants to reach {what} on the server {} in the organization {}. {after} Only continue if you trust where the link came from.",
            link.server, link.org
        ))
    }

    fn set_link_step(&mut self, step: LinkStep) {
        if let Some(pending) = self.link.as_mut() {
            pending.step = step;
            pending.deadline = None;
        }
    }

    /// The flow already holds the next move, so it goes out even if an earlier reply is still settling.
    fn link_request(&mut self, method: &str, path: &str, body: Option<serde_json::Value>) {
        self.busy = false;
        self.request(method, path, body);
    }

    fn link_error(&mut self, message: String) {
        self.link = None;
        self.error = Some(message);
    }

    pub fn confirm_link(&mut self) {
        if self.link_step() != Some(LinkStep::Confirm) {
            self.modal = None;
            return;
        }
        self.modal = None;
        if self.phase == ViewState::Offline {
            self.link = None;
            self.begin_setup();
            self.error = Some(
                "Reoclo's background service is not running. Install it here, then open the link again."
                    .into(),
            );
        } else if self.signed_in() {
            self.link_orgs();
        } else {
            self.set_link_step(LinkStep::SignIn);
            self.link_request("POST", "/v1/auth/login", None);
        }
    }

    /// The sign-in the link started finished.
    pub(super) fn link_signed_in(&mut self) {
        if self.link_step() == Some(LinkStep::SignIn) {
            self.link_orgs();
        }
    }

    fn link_orgs(&mut self) {
        self.set_link_step(LinkStep::Orgs);
        self.link_request("GET", "/v1/orgs", None);
    }

    pub(super) fn link_reply(&mut self, request: &Request) {
        let Some(step) = self.link_step() else {
            return;
        };
        let (method, path) = (request.method.as_str(), request.path.as_str());
        match step {
            LinkStep::Orgs if method == "GET" && path == "/v1/orgs" => self.link_after_orgs(),
            LinkStep::Servers if method == "GET" && path.ends_with("/servers") => {
                self.link_after_servers()
            }
            LinkStep::Reaching if method == "POST" && path.starts_with("/v1/tunnels") => {
                self.link_check_tunnel()
            }
            _ => {}
        }
    }

    /// A request the link was waiting on failed, and its error is already on screen.
    pub(super) fn link_failed(&mut self, request: &Request) {
        let Some(step) = self.link_step() else {
            return;
        };
        let (method, path) = (request.method.as_str(), request.path.as_str());
        let owned = match step {
            LinkStep::SignIn => method == "POST" && path == "/v1/auth/login",
            LinkStep::Orgs => method == "GET" && path == "/v1/orgs",
            LinkStep::Servers => method == "GET" && path.ends_with("/servers"),
            LinkStep::Secure => method == "POST" && path == "/v1/ca/trust",
            LinkStep::Reaching => method == "POST" && path.starts_with("/v1/tunnels"),
            LinkStep::Confirm => false,
        };
        if owned {
            self.link = None;
        }
    }

    fn link_after_orgs(&mut self) {
        let Some(pending) = self.link.as_ref() else {
            return;
        };
        let org = pending.link.org.clone();
        let Some(found) = self.orgs.iter().find(|candidate| candidate.slug == org) else {
            self.link_error(format!(
                "Your Reoclo account does not have access to {org}."
            ));
            return;
        };
        let (tenant_id, path) = (
            found.id.clone(),
            format!("/v1/orgs/{}/servers", segment(&org)),
        );
        if let Some(pending) = self.link.as_mut() {
            pending.tenant_id = tenant_id;
        }
        self.set_link_step(LinkStep::Servers);
        self.link_request("GET", &path, None);
    }

    fn link_after_servers(&mut self) {
        let Some(pending) = self.link.as_ref() else {
            return;
        };
        let (wanted, org) = (pending.link.server.clone(), pending.link.org.clone());
        let Some(found) = self
            .servers
            .iter()
            .find(|server| server.slug == wanted || server.id == wanted)
        else {
            self.link_error(format!(
                "Reoclo could not find a server called {wanted} in {org}."
            ));
            return;
        };
        let (id, slug) = (found.id.clone(), found.slug.clone());
        if let Some(pending) = self.link.as_mut() {
            pending.server_id = id;
            pending.server_slug = slug;
        }
        self.link_proceed();
    }

    /// The link is valid and its server is known: secure HTTPS if needed, then reach the tunnel.
    pub(super) fn link_proceed(&mut self) {
        let Some(pending) = self.link.clone() else {
            return;
        };
        let link = &pending.link;
        if link.kind == LinkKind::Https && !self.ca_ready() {
            self.set_link_step(LinkStep::Secure);
            self.secure_next();
            return;
        }
        let target = link.target();
        let existing = self
            .model
            .tunnels
            .iter()
            .find(|tunnel| {
                tunnel.server_id == pending.server_id
                    && tunnel.kind == link.kind.as_str()
                    && tunnel.target.as_deref() == Some(target.as_str())
            })
            .cloned();
        self.set_link_step(LinkStep::Reaching);
        if let Some(tunnel) = existing {
            if let Some(pending) = self.link.as_mut() {
                pending.tunnel = tunnel.name.clone();
            }
            match tunnel.state {
                TunnelState::Ready | TunnelState::Active => self.link_check_tunnel(),
                TunnelState::Connecting | TunnelState::Reconnecting => {}
                _ => self.link_request(
                    "POST",
                    &format!("/v1/tunnels/{}/up", segment(&tunnel.name)),
                    None,
                ),
            }
            return;
        }
        let name = self.unique_tunnel_name(&link.name);
        let mut body = json!({
            "name": name,
            "org": link.org,
            "server": pending.server_slug,
            "server_id": pending.server_id,
            "kind": link.kind.as_str(),
            "via": "reoclo",
            "tenant_id": pending.tenant_id,
            "target": target,
        });
        match link.kind {
            LinkKind::Https => {
                body["hostname"] = json!(default_hostname(&name, &pending.server_slug));
            }
            LinkKind::Tcp => body["local_port"] = json!(link.port),
        }
        if let Some(pending) = self.link.as_mut() {
            pending.tunnel = name;
        }
        self.link_request("POST", "/v1/tunnels", Some(body));
    }

    fn unique_tunnel_name(&self, base: &str) -> String {
        let taken = |name: &str| self.model.tunnels.iter().any(|tunnel| tunnel.name == name);
        if !taken(base) {
            return base.to_owned();
        }
        let mut number = 2u32;
        loop {
            let suffix = format!("-{number}");
            let mut stem = base.to_owned();
            stem.truncate(63 - suffix.len());
            let name = format!("{}{suffix}", stem.trim_end_matches('-'));
            if !taken(&name) {
                return name;
            }
            number += 1;
        }
    }

    /// Reads the tunnel the link is waiting on. Ready already means it is listening, and a
    /// session opens when the browser or client connects, so Ready and Active both count.
    pub(super) fn link_check_tunnel(&mut self) {
        let Some(pending) = self.link.as_ref() else {
            return;
        };
        if pending.step != LinkStep::Reaching {
            return;
        }
        let kind = pending.link.kind;
        let Some(tunnel) = self
            .model
            .tunnels
            .iter()
            .find(|tunnel| tunnel.name == pending.tunnel)
            .cloned()
        else {
            return;
        };
        if failure(tunnel.state) {
            let reason = if tunnel.message.is_empty() {
                tunnel.state.label()
            } else {
                tunnel.message.as_str()
            };
            self.link_error(format!("{} could not be reached: {reason}.", tunnel.name));
            return;
        }
        if !matches!(tunnel.state, TunnelState::Ready | TunnelState::Active) {
            return;
        }
        self.link = None;
        self.model.selected = Some(tunnel.name.clone());
        self.table.select(&tunnel.name);
        match kind {
            LinkKind::Https => match tunnel.url {
                Some(url) => {
                    self.notice = Some(format!("Opening {}.", tunnel.name));
                    self.effects.push(Effect::OpenUrl(url));
                }
                None => {
                    self.error = Some(format!("{} is up but has no address yet.", tunnel.name));
                }
            },
            LinkKind::Tcp => {
                let endpoint = tunnel
                    .endpoint
                    .clone()
                    .or_else(|| tunnel.local_port.map(|port| format!("127.0.0.1:{port}")));
                match endpoint {
                    Some(endpoint) => {
                        self.notice =
                            Some(format!("Copied {endpoint}. Connect to it from this Mac."));
                        self.effects.push(Effect::Copy(endpoint));
                    }
                    None => {
                        self.error =
                            Some(format!("{} is up but has no local port yet.", tunnel.name));
                    }
                }
            }
        }
    }

    pub fn take_effects(&mut self) -> Vec<Effect> {
        std::mem::take(&mut self.effects)
    }

    fn flush_effects(&mut self, tx: &mut rune::Tx) -> bool {
        let effects = self.take_effects();
        let any = !effects.is_empty();
        for effect in effects {
            match effect {
                Effect::OpenUrl(url) => {
                    if let Err(error) = tx.open_url(&url) {
                        self.error = Some(error.to_string());
                    }
                }
                Effect::Copy(text) => tx.copy(&text),
            }
        }
        any
    }

    /// A step that never hears back drops the link, so a later event cannot open it unasked.
    fn expire_link(&mut self, clock: f64, tx: &mut rune::Tx) -> bool {
        let Some(pending) = self.link.as_mut() else {
            return false;
        };
        let limit = match pending.step {
            LinkStep::Confirm => return false,
            LinkStep::Reaching => REACH_SECONDS,
            _ => SIGN_IN_SECONDS,
        };
        match pending.deadline {
            None => {
                pending.deadline = Some(clock + limit);
                tx.wake_at(clock + limit);
                false
            }
            Some(due) if clock >= due => {
                self.link_error("The link did not finish. Open it again to retry.".into());
                true
            }
            Some(due) => {
                tx.wake_at(due);
                false
            }
        }
    }

    /// Runs before every frame. Returns true when something changed and the view must redraw.
    pub fn poll(&mut self, clock: f64, tx: &mut rune::Tx) -> bool {
        let notice = self.expire_notice(clock, tx);
        let link = self.expire_link(clock, tx);
        let effects = self.flush_effects(tx);
        notice || link || effects
    }
}
