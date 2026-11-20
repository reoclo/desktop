use super::{State, primitives::*};
use crate::{model::TunnelState, theme::*};
use rune::{El, ScrollSpec, col, el, row, text};
use rune_kit::{Button, Kit};

pub fn view(s: &State, k: &Kit) -> El {
    let mut header = row()
        .w_full()
        .px(LG)
        .py(MD)
        .gap(SM)
        .center_y()
        .child(wordmark(k).grow(1.0));
    if s.has_tunnels() {
        let active = s
            .model
            .tunnels
            .iter()
            .filter(|t| t.state == TunnelState::Active)
            .count()
            + s.model
                .foreground
                .iter()
                .filter(|t| t.state == TunnelState::Active)
                .count();
        let failed = s
            .model
            .tunnels
            .iter()
            .filter(|t| t.enabled && t.state.failure())
            .count();
        header = header.child(
            text(if s.current_data() {
                format!("{active} active")
            } else {
                "Unknown".into()
            })
            .font(HINT)
            .color(k.c.text_muted),
        );
        if failed > 0 && s.current_data() {
            header = header.child(indicator(Tone::Danger, &format!("{failed} failed"), k));
        }
    }
    let mut list = col().w_full().padding(LG).gap(LG);
    if s.has_tunnels() {
        if !s.current_data() {
            list = list.child(super::data_state::phase_alert(s, k));
        }
        if let Some(requests) = approvals(s, k).filter(|_| s.current_data()) {
            list = list
                .child(requests)
                .child(el().h(RULE).w_full().fill(k.c.border_base));
        }
        if let Some(running) = terminal_tunnels(s, k).filter(|_| s.current_data()) {
            list = list.child(running);
        }
        for ((org, server), tunnels) in s.model.groups() {
            let mut group = col().w_full().gap(MD).child(
                row()
                    .gap(SM)
                    .center_y()
                    .child(
                        text(if server.is_empty() {
                            "This Mac"
                        } else {
                            server
                        })
                        .font(LABEL)
                        .color(k.c.text_base),
                    )
                    .child(text(org).font(HINT).color(k.c.text_muted)),
            );
            for tunnel in tunnels {
                group = group.child(super::panel_tunnel::view(tunnel, s, k));
            }
            list = list.child(group);
        }
    } else {
        list = col().w_full().child(data_state(s, k));
    }
    let viewport = col()
        .key("panel-scroll")
        .grow(1.0)
        .min_h(0.0)
        .w_full()
        .scroll(ScrollSpec {
            stick_bottom: false,
        })
        .child(list);
    let footer = row()
        .w_full()
        .gap(SM)
        .padding(LG)
        .child(
            Button::new("Open Reoclo")
                .on_press_tx(|s: &mut State, tx| {
                    if let Some(handle) = &s.window_handle {
                        handle.set_dock_icon(true);
                        handle.open();
                        tx.close_window();
                    }
                })
                .primary(k)
                .key("open-reoclo"),
        )
        .child(spacer())
        .child(
            Button::new("Quit Reoclo")
                .help("Tunnels keep running in the background service.")
                .on_press_tx(|_: &mut State, tx| tx.quit())
                .secondary(k)
                .key("quit"),
        );
    let mut panel = col()
        .key("menu-panel")
        .w_full()
        .h_full()
        .fill(k.c.page)
        .border(RULE, k.c.border_base)
        .child(header)
        .child(el().h(RULE).w_full().fill(k.c.border_base))
        .child(k.scroll_bar(viewport));
    let error = if s.has_tunnels() && s.current_data() && s.modal.is_none() {
        s.error.as_deref()
    } else {
        None
    };
    if let Some(error) = error {
        panel = panel
            .child(el().h(RULE).w_full().fill(k.c.border_base))
            .child(alert(error, k).padding(MD));
    } else if let Some(notice) = &s.notice {
        panel = panel
            .child(el().h(RULE).w_full().fill(k.c.border_base))
            .child(
                text(notice)
                    .role("status")
                    .font(HINT)
                    .color(k.c.text_muted)
                    .wrap()
                    .padding(MD),
            );
    }
    if let Some(update) = super::settings::updates_section::panel(s, k) {
        panel = panel
            .child(el().h(RULE).w_full().fill(k.c.border_base))
            .child(update);
    }
    panel
        .child(el().h(RULE).w_full().fill(k.c.border_base))
        .child(footer)
}
