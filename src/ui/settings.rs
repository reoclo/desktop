use super::{State, primitives::*};
use crate::{system::SystemAction, theme::*};
use rune::{El, ScrollSpec, col, el, row, text};
use rune_kit::{Button, ButtonVariant, Kit, Select, Switch, TextInput};
use serde_json::json;

pub fn view(s: &State, k: &Kit, layer: &mut Vec<El>) -> El {
    let appearance = Select::new("appearance", &["system", "light", "dark"])
        .label("Appearance")
        .current(Some(&s.appearance))
        .open(s.open_select.as_deref() == Some("appearance"))
        .on_toggle(|s: &mut State| s.open_select = Some("appearance".into()))
        .on_close(|s: &mut State| s.open_select = None)
        .on_choose(|s: &mut State, value, _| {
            s.appearance = value.into();
            s.open_select = None;
        })
        .build(k, layer);
    let mut body = col().w_full().padding(XL).gap(XL)
        .child(title("Settings", "Preferences for this Mac and its tunnel daemon.", k))
        .child(text("Desktop app").font(HEADING).color(k.c.text_base))
        .child(row().w_full().gap(LG).center_y()
            .child(col().grow(1.0).gap(XS)
                .child(text("Start Reoclo at login").font(BODY).color(k.c.text_base))
                .child(text("Applies at your next sign-in to macOS.").font(LABEL).color(k.c.text_muted)))
            .child(Switch::new(s.start_at_login).label("Start Reoclo at login").hide_label().disabled(s.busy)
                .on_toggle(|s: &mut State| s.run_system(SystemAction::StartAtLogin(!s.start_at_login)))
                .build(k).key("start-at-login")))
        .child(row().w_full().center_y()
            .child(text("Appearance").font(BODY).color(k.c.text_base).grow(1.0))
            .child(appearance))
        .child(el().h(RULE).w_full().fill(k.c.border_base))
        .child(text("Tunnel daemon").font(HEADING).color(k.c.text_base))
        .child(row().w_full().gap(XL)
            .child(col().gap(SM).grow(1.0)
                .child(text("Idle timeout (seconds)").font(LABEL).color(k.c.text_base))
                .child(TextInput::new(&s.idle).label("Idle timeout in seconds").placeholder("600").width(Some(220.0))
                    .on_edit(|s: &mut State| &mut s.idle).build(k).border(RULE, k.c.border_strong).key("idle-timeout"))
                .child(text("Close idle sessions; keep local listeners ready.").font(LABEL).color(k.c.text_muted).wrap()))
            .child(col().gap(SM).grow(1.0)
                .child(text("HTTPS port").font(LABEL).color(k.c.text_base))
                .child(TextInput::new(&s.https_port).label("HTTPS port").placeholder("8443").width(Some(220.0))
                    .on_edit(|s: &mut State| &mut s.https_port).build(k).border(RULE, k.c.border_strong).key("https-port"))
                .child(text("Default 8443. The daemon reports bind failures.").font(LABEL).color(k.c.text_muted).wrap())))
        .child(row().w_full().gap(LG).center_y()
            .child(col().grow(1.0).gap(XS)
                .child(text("Require agent approvals").font(BODY).color(k.c.text_base))
                .child(text("Review reverse tunnels, public binds, hostname changes and tailnet shares.").font(LABEL).color(k.c.text_muted).wrap()))
            .child(Switch::new(s.approvals_required).label("Require agent approvals").hide_label()
                .on_toggle(|s: &mut State| s.approvals_required = !s.approvals_required).build(k).key("require-approvals")))
        .child(Button::new("Save daemon settings").variant(ButtonVariant::Contrast).disabled(s.busy)
            .on_press(|s: &mut State| {
                let Ok(idle) = s.idle.text.parse::<u32>() else { s.error = Some("Enter the idle timeout as a positive number of seconds.".into()); return; };
                let Ok(port) = s.https_port.text.parse::<u16>() else { s.error = Some("Enter an HTTPS port from 1 to 65535.".into()); return; };
                if idle == 0 || port == 0 { s.error = Some("Use a positive idle timeout and an HTTPS port from 1 to 65535.".into()); return; }
                s.request("PATCH", "/v1/settings", Some(json!({"idle_timeout_seconds":idle,"https_port":port,"agent_approvals":if s.approvals_required { "required" } else { "off" }})));
            }).build(k).w(220.0).key("save-settings"))
        .child(field("Connected daemon", &s.model.status.version, k));
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
            .key("settings-scroll")
            .w_full()
            .h_full()
            .scroll(ScrollSpec {
                stick_bottom: false,
            })
            .child(body),
    )
}
