use super::{Page, State, ViewState};
use crate::{fake::Fixture, model::*, reducer::Model};
use rune::TextEdit;

impl State {
    pub fn fixture(page: &str, case: &str) -> Result<Self, serde_json::Error> {
        let f = Fixture::load()?;
        let mut s = Self {
            model: Model {
                status: f.status,
                tunnels: f.tunnels,
                approvals: f.approvals,
                host_entries: f.host_entries,
                selected: Some("grafana".into()),
                ..Default::default()
            },
            orgs: f.orgs,
            servers: f.servers,
            ports: f.ports,
            org: "demo".into(),
            server: "server-1".into(),
            phase: ViewState::Loaded,
            ..Default::default()
        };
        s.page = match page {
            "hosts" => Page::Hosts,
            "certificates" => Page::Certificates,
            "settings" => Page::Settings,
            _ => Page::Tunnels,
        };
        s.panel = page == "panel";
        if page == "add" {
            s.modal = Some("add".into());
            s.name = TextEdit::new("api");
        }
        if page == "edit" {
            s.edit_selected();
        }
        if matches!(
            page,
            "remove" | "discard" | "start" | "setup" | "trust" | "untrust"
        ) {
            s.modal = Some(page.into());
            if page == "discard" {
                s.return_modal = Some("edit".into());
            }
            if page == "setup" {
                s.page = Page::Hosts;
            }
            if matches!(page, "trust" | "untrust") {
                s.page = Page::Certificates;
            }
        }
        s.phase = match case {
            "loading" => ViewState::Loading,
            "empty" => ViewState::Empty,
            "offline" => ViewState::Offline,
            "signed-out" | "device-code" => ViewState::SignedOut,
            "error" => ViewState::Error,
            _ => ViewState::Loaded,
        };
        if case == "empty" {
            s.model.tunnels.clear();
            s.model.approvals.clear();
            s.model.selected = None;
            s.model.host_entries.clear();
            if let Some(hosts) = &mut s.model.status.hosts {
                hosts.installed = false;
                hosts.entries = 0;
            }
            if let Some(ca) = &mut s.model.status.ca {
                ca.exists = false;
                ca.trusted = false;
                ca.not_after = None;
                ca.suffixes.clear();
            }
            if s.page == Page::Settings {
                s.notice = Some("No saved preferences. Default daemon settings are shown.".into());
            }
        }
        if let Ok(state) = serde_json::from_value::<TunnelState>(serde_json::json!(case))
            && state != TunnelState::Unknown
            && case != "loaded"
            && let Some(tunnel) = s.model.tunnels.first_mut()
        {
            tunnel.state = state;
            tunnel.message = state.label().into();
            tunnel.detail = Some("The daemon reported this tunnel state.".into());
            tunnel.action = "Review the state, then retry or sign in as needed.".into();
        }
        if case == "error" {
            s.error = Some(
                "The daemon rejected the request. Check the local daemon log and retry.".into(),
            );
        }
        if case == "device-code" {
            s.model.auth = AuthEvent {
                user_code: Some("DEMO-7294".into()),
                verification_uri: Some("https://auth.reoclo.com/device".into()),
                status: "pending".into(),
                ..Default::default()
            };
        }
        s.table.select("grafana");
        Ok(s)
    }
}
