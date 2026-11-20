use super::{Page, State, ViewState, primitives::*};
use crate::{control::segment, theme::*};
use rune::{El, col, row};
use rune_kit::{Button, Kit};
use std::collections::BTreeMap;

impl State {
    pub fn current_data(&self) -> bool {
        matches!(self.phase, ViewState::Loaded | ViewState::Empty)
    }

    pub fn shared_failures(&self) -> Vec<(&str, Vec<String>)> {
        let mut groups: BTreeMap<&str, Vec<String>> = BTreeMap::new();
        for tunnel in &self.model.tunnels {
            if tunnel.state.failure()
                && let Some(detail) = tunnel.detail.as_deref().filter(|d| !d.is_empty())
            {
                groups.entry(detail).or_default().push(tunnel.name.clone());
            }
        }
        groups
            .into_iter()
            .filter(|(_, names)| names.len() > 1)
            .collect()
    }

    pub fn tunnel_pending(&self, name: &str) -> bool {
        let prefix = format!("/v1/tunnels/{}", segment(name));
        self.pending
            .iter()
            .any(|path| path == &prefix || path.starts_with(&format!("{prefix}/")))
    }

    pub fn approval_pending(&self, id: &str) -> bool {
        let prefix = format!("/v1/approvals/{}/", segment(id));
        self.pending.iter().any(|path| path.starts_with(&prefix))
    }

    pub fn reveal_tunnel(&mut self, name: &str, tx: &mut rune::Tx) {
        self.page = Page::Tunnels;
        self.model.selected = Some(name.into());
        self.table.select(name);
        tx.scroll_into_view("tunnel-header");
    }
}

pub fn shared_alerts(s: &State, k: &Kit) -> El {
    let mut alerts = col().w_full().gap(MD);
    for (index, (detail, names)) in s.shared_failures().into_iter().enumerate() {
        let port = detail
            .strip_prefix("port ")
            .and_then(|rest| rest.split_once(" is already in use, so HTTPS tunnels cannot listen"))
            .and_then(|(port, _)| port.parse::<u16>().ok());
        let message = match port {
            Some(port) => format!(
                "Port {port} is in use, so {} HTTPS tunnels cannot listen.",
                names.len()
            ),
            None => format!("{} tunnels share this failure: {detail}", names.len()),
        };
        let pending = names.iter().any(|name| s.tunnel_pending(name));
        let mut actions = row().gap(SM);
        if port.is_some() {
            actions = actions.child(
                Button::new("Change HTTPS port")
                    .on_press_tx(|s: &mut State, tx| {
                        s.show_page(Page::Settings);
                        tx.focus_when_shown("https-port", true);
                        tx.scroll_into_view("https-port");
                    })
                    .primary(k)
                    .key(format!("change-https-port-{index}")),
            );
        }
        actions = actions.child(
            Button::new("Try again")
                .disabled(pending)
                .on_press(move |s: &mut State| {
                    for name in &names {
                        s.request("POST", &format!("/v1/tunnels/{}/up", segment(name)), None);
                    }
                })
                .secondary(k)
                .key(format!("retry-group-{index}")),
        );
        alerts = alerts.child(
            col()
                .w_full()
                .gap(SM)
                .child(alert(&message, k))
                .child(actions),
        );
    }
    alerts
}
