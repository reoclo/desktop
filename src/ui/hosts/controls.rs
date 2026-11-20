use super::*;
use crate::model::{HostEntry, Hosts};
use rune_kit::{ControlSize, Switch, TextInput};

fn set_manual_hosts(s: &mut State, hosts: Vec<String>) {
    s.request(
        "PATCH",
        "/v1/settings",
        Some(serde_json::json!({ "manual_hosts": hosts })),
    );
}

fn register_host(s: &mut State) {
    let name = s.new_host.text.trim().to_ascii_lowercase();
    if name.is_empty() {
        return;
    }
    let mut hosts = s.settings.manual_hosts.clone();
    if !hosts.contains(&name) {
        hosts.push(name);
        hosts.sort();
    }
    set_manual_hosts(s, hosts);
}

/// Names you registered yourself can be removed here; tunnel names go with their tunnel.
pub(super) fn remove_manual(entry: &HostEntry, s: &State, k: &Kit) -> El {
    if entry.source != "manual" {
        return el();
    }
    let name = entry.hostname.clone();
    Button::new("Remove")
        .control_size(ControlSize::Small)
        .disabled(s.busy)
        .on_press(move |s: &mut State| {
            let hosts = s
                .settings
                .manual_hosts
                .iter()
                .filter(|h| **h != name)
                .cloned()
                .collect();
            set_manual_hosts(s, hosts);
        })
        .build(k)
        .border(RULE, k.c.border_strong)
        .key(format!("remove-host-{}", entry.hostname))
}

pub(super) fn register_row(s: &State, k: &Kit) -> El {
    row()
        .w_full()
        .gap(SM)
        .center_y()
        .mt(MD)
        .child(
            TextInput::new(&s.new_host)
                .label("Hostname to register")
                .placeholder("app.localhost")
                .width(Some(FORM_COLUMN))
                .on_edit(|s: &mut State| &mut s.new_host)
                .on_submit(|s: &mut State, _| register_host(s))
                .build(k)
                .border(RULE, k.c.border_strong)
                .key("new-host"),
        )
        .child(
            Button::new("Register hostname")
                .disabled(s.busy || s.new_host.text.trim().is_empty())
                .on_press(register_host)
                .secondary(k)
                .key("register-host"),
        )
}

pub(super) fn relay_detail(hosts: &Hosts) -> &'static str {
    match (hosts.relay.enabled, hosts.relay.listening) {
        (_, true) => "HTTPS URLs need no port.",
        (true, false) => "Another program answers on port 443, so URLs keep the daemon's port.",
        _ => "HTTPS URLs include the daemon's port.",
    }
}

pub(super) fn relay_controls(hosts: &Hosts, s: &State, k: &Kit) -> El {
    let status = match (hosts.relay.enabled, hosts.relay.listening) {
        (_, true) => indicator(Tone::Success, "Listening", k),
        (true, false) => indicator(Tone::Warning, "Port 443 in use", k),
        _ => indicator(Tone::Inactive, "Off", k),
    };
    let mut controls = row().gap(LG).center_y().child(status);
    if hosts.installed {
        let enabled = hosts.relay.enabled;
        controls = controls.child(
            Switch::new(enabled)
                .label("Port 443 relay")
                .hide_label()
                .disabled(s.busy)
                .on_toggle(move |s: &mut State| {
                    s.modal = Some(if enabled { "relay-off" } else { "relay-on" }.into())
                })
                .build(k)
                .key("relay-toggle"),
        );
    }
    controls
}
