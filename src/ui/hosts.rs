mod controls;
use super::{State, primitives::*};
use crate::theme::*;
use controls::{register_row, relay_controls, relay_detail, remove_manual};
use rune::{El, ScrollSpec, col, el, row, text};
use rune_kit::{Button, Kit};

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
        .max_w(CONTENT_MAX)
        .padding(if s.compact { LG } else { XL })
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
                        .child(text("Hosts helper").role("heading").font(HEADING).color(k.c.text_base))
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
                .child(text("Managed entries").role("heading").font(HEADING).color(k.c.text_base).mb(SM));
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
