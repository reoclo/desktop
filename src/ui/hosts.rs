use super::{State, primitives::*};
use crate::theme::*;
use rune::{El, ScrollSpec, col, el, row, text};
use rune_kit::{Button, ButtonVariant, Kit};

pub fn view(s: &State, k: &Kit) -> El {
    let hosts = s.model.status.hosts.as_ref();
    let installed = hosts.is_some_and(|h| h.installed);
    let mut body = col().w_full().padding(XL).gap(XL)
        .child(title("Hosts", "Local names for services on your servers.", k))
        .child(row().gap(XXL).center_y()
            .child(col().grow(1.0).gap(SM)
                .child(text("Hosts helper").font(HEADING).color(k.c.text_base))
                .child(text(if installed { "Installed" } else { "Not installed" }).font(LABEL).color(if installed { k.c.success.fg } else { k.c.text_muted })))
            .child(Button::new("Set up helper").variant(ButtonVariant::Contrast)
                .on_press(|s: &mut State| s.modal = Some("setup".into())).build(k).key("setup-hosts")))
        .child(text("The helper maintains a Reoclo block in /etc/hosts. Every managed name points to 127.0.0.1 and ::1. Setup opens a macOS administrator prompt.")
            .font(BODY).color(k.c.text_muted).wrap().max_w(640.0))
        .child(row().gap(SM).center_y()
            .child(field("Run from Terminal", "reoclo hosts setup", k).grow(1.0))
            .child(Button::new("Copy command").on_press_tx(|s: &mut State, tx| {
                tx.copy("reoclo hosts setup"); s.notice = Some("Setup command copied.".into());
            }).build(k).key("copy-hosts-command")))
        .child(el().h(RULE).w_full().fill(k.c.border_base))
        .child(text("Managed entries").font(HEADING).color(k.c.text_base));
    match hosts {
        Some(hosts) => {
            if s.model.host_entries.is_empty() {
                body = body.child(text("No managed hostnames. Add an HTTPS tunnel to create one.").font(BODY).color(k.c.text_muted).wrap())
                    .child(Button::new("Add tunnel").on_press(|s: &mut State| s.begin_add()).build(k));
            }
            for entry in &s.model.host_entries {
                body = body.child(row().w_full().gap(LG).py(SM)
                    .child(text(&entry.hostname).font(MONO).color(k.c.text_base).wrap().selectable(true).grow(1.0))
                    .child(text("127.0.0.1 / ::1").font(MONO).color(k.c.text_muted)));
            }
            body = body.child(field("Port 443 relay", if hosts.relay.listening { "Listening" } else { "Off. HTTPS uses the daemon's configured port." }, k));
            if let Some(error) = &hosts.error { body = body.child(text(error).role("alert").font(BODY).color(k.c.danger.fg).wrap()); }
        }
        None => body = body.child(text("This daemon does not report hosts. Update the bundled CLI to use managed hostnames.").font(BODY).color(k.c.text_muted).wrap()),
    }
    if let Some(message) = s.error.as_ref().or(s.notice.as_ref()) {
        body = body.child(
            text(message)
                .role("status")
                .font(BODY)
                .color(k.c.text_base)
                .wrap(),
        );
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
