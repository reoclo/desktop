use super::{State, primitives::*, tunnel_rows::TunnelRow};
use crate::{
    control::segment,
    model::{ForegroundTunnel, TunnelState, TunnelView},
    theme::*,
};
use rune::{Cursor, El, col, el, row, text};
use rune_kit::{
    Button, ButtonVariant, ControlSize, IconButton, IconButtonVariant, Kit, Switch, glyphs,
};

impl State {
    /// The panel's one list: enabled tunnels that failed first, then the main table's order.
    pub(super) fn panel_rows(&self) -> Vec<TunnelRow<'_>> {
        let failed = |row: &TunnelRow<'_>| matches!(row, TunnelRow::Saved(t) if self.current_data() && t.enabled && t.state.failure());
        let (mut rows, rest): (Vec<_>, Vec<_>) = (0..self.tunnel_row_count())
            .filter_map(|index| self.tunnel_row(index))
            .partition(failed);
        rows.extend(rest);
        rows
    }
}

/// One line: state mark, name, endpoint, then the switch or Stop. A failure adds its reason.
pub fn view(tunnel: &TunnelRow<'_>, s: &State, k: &Kit) -> El {
    match *tunnel {
        TunnelRow::Saved(t) => saved(t, s, k),
        TunnelRow::Terminal(t, index) => terminal(t, index, s, k),
    }
}

struct Line {
    key: String,
    state: TunnelState,
    name: El,
    endpoint: El,
    /// Only web tunnels that are listening have one; the endpoint takes its place otherwise.
    open: Option<El>,
    action: El,
}

/// One gutter between every column, so the name and its tag never reach the endpoint. The
/// endpoint's own inset doubles the gap its text keeps from the columns either side.
fn line(l: Line, k: &Kit) -> El {
    row()
        .key(l.key)
        .w_full()
        .h(PANEL_ROW)
        .gap(SM)
        .center_y()
        .child(state_mark(l.state, k))
        .child(l.name.w(PANEL_NAME).min_w(0.0))
        .child(l.endpoint.grow(1.0).min_w(0.0))
        .children(l.open)
        .child(
            row()
                .w(PANEL_ACTION)
                .justify_end()
                .center_y()
                .child(l.action),
        )
}

fn note(message: &str, color: rune::Color) -> El {
    text(message)
        .font(HINT)
        .color(color)
        .truncate()
        .selectable(true)
        .help_when_truncated(message)
        .w_full()
        .min_w(0.0)
        .pl(LG)
        .pb(XS)
}

fn reveal(content: El, target: String, label: String, k: &Kit) -> El {
    content
        .role("button")
        .label(label)
        .focusable(true)
        .focus_ring(RULE, XS, k.c.border_focus)
        .cursor(Cursor::Pointer)
        .on_press_tx(move |s: &mut State, tx| {
            s.reveal_tunnel(&target, tx);
            if let Some(handle) = &s.window_handle {
                let target = target.clone();
                handle.update(move |s, tx| s.reveal_tunnel(&target, tx));
                handle.set_dock_icon(true);
                handle.open();
                tx.close_window();
            }
        })
}

fn copy(name: &str, endpoint: &str, k: &Kit) -> El {
    let shown = endpoint.strip_prefix("https://").unwrap_or(endpoint);
    if endpoint == "Not listening" {
        return text(shown)
            .font(HINT)
            .color(k.c.text_muted)
            .truncate()
            .px(XS);
    }
    let value = endpoint.to_owned();
    text(shown)
        .font(MONO)
        .color(k.c.text_muted)
        .truncate()
        .role("button")
        .label(format!("Copy {name} endpoint"))
        .help(format!("Copy {endpoint}"))
        .focusable(true)
        .focus_ring(RULE, XS, k.c.border_focus)
        .cursor(Cursor::Pointer)
        .hover(k.c.hover)
        .radius(k.radius.sm)
        .px(XS)
        .on_press_tx(move |s: &mut State, tx| {
            tx.copy(&value);
            s.notice = Some("Endpoint copied.".into());
        })
        .key(format!("copy-{name}"))
}

fn saved(t: &TunnelView, s: &State, k: &Kit) -> El {
    let current = s.current_data();
    let state = match (current, t.retry.is_some()) {
        (false, _) => TunnelState::Unknown,
        (true, true) => TunnelState::Reconnecting,
        (true, false) => t.state,
    };
    let mut open = None;
    if t.state.available()
        && current
        && let Some(url) = t.url.clone()
    {
        open = Some(
            IconButton::new(&glyphs::ARROW_RIGHT)
                .variant(IconButtonVariant::GhostMuted)
                .control_size(ControlSize::Small)
                .label(format!("Open {} in browser", t.name))
                .on_press_tx(move |s: &mut State, tx| {
                    if let Err(error) = tx.open_url(&url) {
                        s.error = Some(error.to_string());
                    }
                })
                .build(k)
                .key(format!("open-{}", t.name)),
        );
    }
    let action = if current {
        let path = format!(
            "/v1/tunnels/{}/{}",
            segment(&t.name),
            if t.enabled { "down" } else { "up" }
        );
        Switch::new(t.enabled)
            .label(format!("Run {}", t.name))
            .hide_label()
            .disabled(s.tunnel_pending(&t.name))
            .on_toggle(move |s: &mut State| s.request("POST", &path, None))
            .build(k)
            .key(format!("toggle-{}", t.name))
    } else {
        el()
    };
    let name = text(&t.name)
        .font(BODY)
        .color(k.c.text_base)
        .truncate()
        .help_when_truncated(&t.name);
    let mut item = col()
        .key(format!("panel-row-{}", t.name))
        .w_full()
        .child(line(
            Line {
                key: format!("panel-line-{}", t.name),
                state,
                name: reveal(
                    name,
                    t.name.clone(),
                    format!("Show {} in Reoclo", t.name),
                    k,
                )
                .key(format!("show-tunnel-{}", t.name)),
                endpoint: copy(&t.name, t.endpoint(), k),
                open,
                action,
            },
            k,
        ));
    if current && t.enabled && t.state.failure() {
        item = item.child(note(
            t.detail.as_deref().unwrap_or(&t.message),
            k.c.danger.fg,
        ));
    }
    if t.share.as_ref().is_some_and(|share| share.tailnet) {
        item = item.child(note("Shared with tailnet", k.c.info.fg));
    }
    item
}

fn terminal(t: &ForegroundTunnel, index: usize, s: &State, k: &Kit) -> El {
    let key = format!("terminal:{}:{index}", t.id);
    // The server gives way to the tag, so both stay inside the name column.
    let name = row()
        .gap(XS)
        .center_y()
        .child(
            text(&t.server)
                .font(BODY)
                .color(k.c.text_base)
                .truncate()
                .help_when_truncated(&t.server)
                .min_w(0.0)
                .shrink(),
        )
        .child(text("Terminal").font(HINT).color(k.c.text_muted));
    let endpoint = match t.forwards.get(index) {
        Some(forward) => copy(&t.server, &forward.listen, k).help(format!(
            "Copy {}. Forwards to {} on {}.",
            forward.listen, forward.target, t.server
        )),
        None => text("No forwards").font(HINT).color(k.c.text_muted).px(XS),
    };
    let action = if s.current_data() {
        let id = t.id.clone();
        Button::new("Stop")
            .control_size(ControlSize::Small)
            .help(format!("Stop the terminal tunnel to {}", t.server))
            .on_press(move |s: &mut State| {
                s.stop_target = Some(id.clone());
                s.modal = Some("stop-terminal".into());
            })
            .variant(ButtonVariant::Danger)
            .build(k)
            .border(RULE, k.c.danger.fg)
            .key(if index == 0 {
                format!("stop-terminal-{}", t.id)
            } else {
                format!("stop-terminal-{}-{index}", t.id)
            })
    } else {
        el()
    };
    col().key(format!("panel-row-{key}")).w_full().child(line(
        Line {
            key: format!("panel-line-{key}"),
            state: if s.current_data() {
                t.state
            } else {
                TunnelState::Unknown
            },
            name: reveal(
                name,
                key,
                format!("Show the terminal tunnel to {} in Reoclo", t.server),
                k,
            ),
            endpoint,
            open: None,
            action,
        },
        k,
    ))
}
