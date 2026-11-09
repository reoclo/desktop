use super::{State, primitives::*};
use crate::{control::segment, model::TunnelState, theme::*};
use rune::{El, ScrollSpec, col, el, row, text};
use rune_kit::{Button, IconButton, IconButtonVariant, Kit, Switch, glyphs};

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
        header = header.child(
            text(format!("{active} active"))
                .font(HINT)
                .color(k.c.text_muted),
        );
    }
    let mut list = col().w_full().padding(LG).gap(LG);
    if s.has_tunnels() {
        if let Some(requests) = approvals(s, k) {
            list = list
                .child(requests)
                .child(el().h(RULE).w_full().fill(k.c.border_base));
        }
        if let Some(running) = terminal_tunnels(s, k) {
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
                let path = format!(
                    "/v1/tunnels/{}/{}",
                    segment(&tunnel.name),
                    if tunnel.enabled { "down" } else { "up" }
                );
                let copy = tunnel.endpoint().to_owned();
                // Copy is last so it keeps its column on rows without a URL.
                let mut controls = row().gap(XS).center_y();
                if let Some(url) = tunnel.url.clone() {
                    controls = controls.child(
                        IconButton::new(&glyphs::ARROW_RIGHT)
                            .variant(IconButtonVariant::GhostMuted)
                            .label("Open in browser")
                            .on_press_tx(move |s: &mut State, tx| {
                                if let Err(error) = tx.open_url(&url) {
                                    s.error = Some(error.to_string());
                                }
                            })
                            .build(k)
                            .key(format!("open-{}", tunnel.name)),
                    );
                }
                let controls = controls.child(
                    IconButton::new(&glyphs::COPY)
                        .variant(IconButtonVariant::GhostMuted)
                        .label(if tunnel.url.is_some() {
                            "Copy URL"
                        } else {
                            "Copy endpoint"
                        })
                        .on_press_tx(move |s: &mut State, tx| {
                            tx.copy(&copy);
                            s.notice = Some("Endpoint copied.".into());
                        })
                        .build(k)
                        .key(format!("copy-{}", tunnel.name)),
                );
                // The open action already says this is a web address, so the
                // scheme can give its width to the hostname and port.
                let endpoint = tunnel.endpoint();
                let shown = endpoint.strip_prefix("https://").unwrap_or(endpoint);
                let mut item = col()
                    .w_full()
                    .gap(XS)
                    .child(
                        row()
                            .w_full()
                            .gap(SM)
                            .center_y()
                            .child(
                                text(&tunnel.name)
                                    .font(BODY)
                                    .color(k.c.text_base)
                                    .truncate()
                                    .help_when_truncated(&tunnel.name)
                                    .grow(1.0)
                                    .min_w(0.0),
                            )
                            .child(status(tunnel.state, k))
                            .child(controls)
                            .child(
                                Switch::new(tunnel.enabled)
                                    .label(format!("{} tunnel up", tunnel.name))
                                    .hide_label()
                                    .disabled(s.busy)
                                    .on_toggle(move |s: &mut State| s.request("POST", &path, None))
                                    .build(k)
                                    .key(format!("toggle-{}", tunnel.name)),
                            ),
                    )
                    .child(
                        text(shown)
                            .font(MONO)
                            .color(k.c.text_muted)
                            .truncate()
                            .selectable(true)
                            .help_when_truncated(endpoint)
                            .w_full()
                            .min_w(0.0),
                    );
                if tunnel.share.as_ref().is_some_and(|share| share.tailnet) {
                    item = item.child(indicator(Tone::Info, "Shared with tailnet", k));
                }
                group = group.child(item);
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
            Button::new("Quit")
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
    let error = if s.has_tunnels() && s.modal.is_none() {
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
    panel
        .child(el().h(RULE).w_full().fill(k.c.border_base))
        .child(footer)
}
