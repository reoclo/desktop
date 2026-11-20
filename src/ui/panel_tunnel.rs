use super::{State, primitives::*};
use crate::{
    control::segment,
    model::{TunnelState, TunnelView},
    theme::*,
};
use rune::{El, col, el, row, text};
use rune_kit::{IconButton, IconButtonVariant, Kit, Switch, glyphs};

pub fn view(t: &TunnelView, s: &State, k: &Kit) -> El {
    let path = format!(
        "/v1/tunnels/{}/{}",
        segment(&t.name),
        if t.enabled { "down" } else { "up" }
    );
    let copy = t.endpoint().to_owned();
    let mut open = el().size(XL, XL);
    if t.state.available()
        && s.current_data()
        && let Some(url) = t.url.clone()
    {
        open = IconButton::new(&glyphs::ARROW_RIGHT)
            .variant(IconButtonVariant::GhostMuted)
            .label(format!("Open {} in browser", t.name))
            .on_press_tx(move |s: &mut State, tx| {
                if let Err(error) = tx.open_url(&url) {
                    s.error = Some(error.to_string());
                }
            })
            .build(k)
            .key(format!("open-{}", t.name));
    }
    let copy_button = IconButton::new(&glyphs::COPY)
        .variant(IconButtonVariant::GhostMuted)
        .label(format!("Copy {} endpoint", t.name))
        .on_press_tx(move |s: &mut State, tx| {
            tx.copy(&copy);
            s.notice = Some("Endpoint copied.".into());
        })
        .build(k)
        .key(format!("copy-{}", t.name));
    let name = t.name.clone();
    let name_button = text(&t.name)
        .role("button")
        .label(format!("Show {} in Reoclo", t.name))
        .font(BODY)
        .color(k.c.text_base)
        .truncate()
        .help_when_truncated(&t.name)
        .grow(1.0)
        .min_w(0.0)
        .focusable(true)
        .focus_ring(RULE, XS, k.c.border_focus)
        .on_press_tx(move |s: &mut State, tx| {
            s.reveal_tunnel(&name, tx);
            if let Some(handle) = &s.window_handle {
                let name = name.clone();
                handle.update(move |s, tx| s.reveal_tunnel(&name, tx));
                handle.set_dock_icon(true);
                handle.open();
                tx.close_window();
            }
        })
        .key(format!("show-tunnel-{}", t.name));
    let mut controls = row()
        .w_full()
        .gap(XS)
        .center_y()
        .child(name_button)
        .child(
            status(
                if s.current_data() {
                    t.state
                } else {
                    TunnelState::Unknown
                },
                k,
            )
            .w(COLUMN_MIN),
        )
        .child(open)
        .child(copy_button);
    if s.current_data() {
        controls = controls.child(
            Switch::new(t.enabled)
                .label(format!("Run {}", t.name))
                .hide_label()
                .disabled(s.tunnel_pending(&t.name))
                .on_toggle(move |s: &mut State| s.request("POST", &path, None))
                .build(k)
                .key(format!("toggle-{}", t.name)),
        );
    }
    let failed = t.state.failure() && s.current_data();
    let shown = if failed {
        t.detail.as_deref().unwrap_or(&t.message)
    } else {
        t.endpoint()
            .strip_prefix("https://")
            .unwrap_or(t.endpoint())
    };
    let mut item = col().w_full().gap(XS).child(controls).child(
        text(shown)
            .font(if failed || shown == "Not listening" {
                HINT
            } else {
                MONO
            })
            .color(if failed {
                k.c.danger.fg
            } else {
                k.c.text_muted
            })
            .truncate()
            .selectable(true)
            .help_when_truncated(shown)
            .w_full()
            .min_w(0.0),
    );
    if t.share.as_ref().is_some_and(|share| share.tailnet) {
        item = item.child(indicator(Tone::Info, "Shared with tailnet", k));
    }
    item
}
