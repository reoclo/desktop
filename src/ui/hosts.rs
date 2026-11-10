use super::{State, primitives::*};
use crate::{
    model::{HostEntry, Hosts},
    theme::*,
};
use rune::{El, ScrollSpec, col, el, row, text};
use rune_kit::{Button, ControlSize, Kit, Switch, TextInput};

pub fn view(s: &State, k: &Kit) -> El {
    let hosts = s.model.status.hosts.as_ref();
    let installed = hosts.is_some_and(|h| h.installed);
    let setup = Button::new(if installed {
        "Reinstall helper"
    } else {
        "Set up helper"
    })
    .on_press(|s: &mut State| s.modal = Some("setup".into()));
    let mut body = col()
        .w_full()
        .padding(XL)
        .gap(XL)
        .child(title("Hosts", None, k));
    if let Some(error) = s.error.as_ref().filter(|_| s.modal.is_none()) {
        body = body.child(alert(error, k));
    }
    let mut helper = col()
        .w_full()
        .gap(MD)
        .child(
            row()
                .w_full()
                .gap(LG)
                .center_y()
                .child(
                    col()
                        .grow(1.0)
                        .min_w(0.0)
                        .gap(SM)
                        .child(text("Hosts helper").font(HEADING).color(k.c.text_base))
                        .child(if installed {
                            indicator(Tone::Success, "Installed", k)
                        } else {
                            indicator(Tone::Inactive, "Not installed", k)
                        }),
                )
                .child(
                    if installed {
                        setup.secondary(k)
                    } else {
                        setup.primary(k)
                    }
                    .key("setup-hosts"),
                ),
        )
        .child(
            text("The helper maintains a Reoclo block in /etc/hosts. Every managed name points to 127.0.0.1 and ::1. Setup opens a macOS administrator prompt.")
                .font(BODY)
                .color(k.c.text_muted)
                .wrap()
                .max_w(640.0),
        );
    if !installed {
        helper = helper.child(
            row()
                .w_full()
                .gap(LG)
                .center_y()
                .child(field("Run from Terminal", "reoclo hosts setup", k).grow(1.0))
                .child(
                    Button::new("Copy command")
                        .on_press_tx(|s: &mut State, tx| {
                            tx.copy("reoclo hosts setup");
                            s.notice = Some("Setup command copied.".into());
                        })
                        .secondary(k)
                        .key("copy-hosts-command"),
                ),
        );
    }
    body = body
        .child(helper)
        .child(el().h(RULE).w_full().fill(k.c.border_base));
    match hosts {
        Some(hosts) => {
            let mut entries = col()
                .w_full()
                .child(text("Managed entries").font(HEADING).color(k.c.text_base).mb(SM));
            if s.model.host_entries.is_empty() {
                entries = entries
                    .gap(MD)
                    .child(
                        text("No managed hostnames. Register one below, or add an HTTPS tunnel.")
                            .font(BODY)
                            .color(k.c.text_muted)
                            .wrap(),
                    )
                    .child(row().child(
                        Button::new("Add tunnel")
                            .on_press(|s: &mut State| s.begin_add())
                            .secondary(k)
                            .key("hosts-add-tunnel"),
                    ));
            }
            for entry in &s.model.host_entries {
                entries = entries
                    .child(el().h(RULE).w_full().fill(k.c.border_base))
                    .child(
                        row()
                            .w_full()
                            .gap(LG)
                            .py(MD)
                            .center_y()
                            .child(
                                text(&entry.hostname)
                                    .font(MONO)
                                    .color(k.c.text_base)
                                    .wrap()
                                    .selectable(true)
                                    .grow(1.0)
                                    .min_w(0.0),
                            )
                            .child(text("127.0.0.1, ::1").font(MONO).color(k.c.text_muted))
                            .child(remove_manual(entry, s, k)),
                    );
            }
            entries = entries.child(register_row(s, k));
            body = body.child(entries).child(
                row()
                    .w_full()
                    .gap(LG)
                    .center_y()
                    .child(
                        col()
                            .grow(1.0)
                            .min_w(0.0)
                            .gap(XS)
                            .child(text("Port 443 relay").font(BODY).color(k.c.text_base))
                            .child(
                                text(relay_detail(hosts))
                                .font(HINT)
                                .color(k.c.text_muted),
                            ),
                    )
                    .child(relay_controls(hosts, s, k)),
            );
            if let Some(error) = &hosts.error {
                body = body.child(alert(error, k));
            }
        }
        None => {
            body = body.child(
                text("This daemon does not report hosts. Update the reoclo CLI and restart the daemon to use managed hostnames.")
                    .font(BODY)
                    .color(k.c.text_muted)
                    .wrap(),
            )
        }
    }
    k.scroll_bar(
        col()
            .key("hosts-scroll")
            .w_full()
            .h_full()
            .scroll(ScrollSpec {
                stick_bottom: false,
            })
            .child(body),
    )
}

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
fn remove_manual(entry: &HostEntry, s: &State, k: &Kit) -> El {
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

fn register_row(s: &State, k: &Kit) -> El {
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

fn relay_detail(hosts: &Hosts) -> &'static str {
    match (hosts.relay.enabled, hosts.relay.listening) {
        (_, true) => "HTTPS URLs need no port.",
        (true, false) => "Another program answers on port 443, so URLs keep the daemon's port.",
        _ => "HTTPS URLs include the daemon's port.",
    }
}

fn relay_controls(hosts: &Hosts, s: &State, k: &Kit) -> El {
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
