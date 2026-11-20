use super::{State, primitives::*, traffic_graph};
use crate::{model::TunnelState, theme::*};
use rune::{El, ScrollSpec, Shape, col, el, row, text};
use rune_kit::{Button, Kit};

pub fn view(s: &State, k: &Kit) -> El {
    let mut list = col().key("panel-list").w_full().px(LG).py(SM);
    if s.has_tunnels() {
        if !s.current_data() {
            list = list.child(super::data_state::phase_alert(s, k).pb(SM));
        }
        let mut attention = col().w_full().gap(MD);
        let mut waiting = false;
        if let Some(requests) = approvals(s, k).filter(|_| s.current_data()) {
            attention = attention.child(requests);
            waiting = true;
        }
        if let Some(update) = super::settings::updates_section::panel(s, k) {
            attention = attention.child(update);
            waiting = true;
        }
        if waiting {
            list = list
                .child(attention.py(XS))
                .child(el().h(RULE).w_full().fill(k.c.border_base).my(SM));
        }
        for tunnel in s.panel_rows() {
            list = list.child(super::panel_tunnel::view(&tunnel, s, k));
        }
    } else {
        list = col().w_full().child(data_state(s, k));
    }
    // Sized by its rows, and the part that gives way once the panel reaches its height.
    let viewport = col()
        .key("panel-scroll")
        .shrink()
        .min_h(0.0)
        .w_full()
        .scroll(ScrollSpec {
            stick_bottom: false,
        })
        .child(list);
    let footer = row()
        .w_full()
        .gap(SM)
        .px(LG)
        .py(MD)
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
    // The window fits this column, which grows with its list up to the panel's height.
    let mut panel = col()
        .key("menu-panel")
        .w_full()
        .max_h(PANEL_HEIGHT)
        .fill(k.c.page)
        .border(RULE, k.c.border_base)
        .child(header(s, k))
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
    panel
        .child(el().h(RULE).w_full().fill(k.c.border_base))
        .child(footer)
}

fn header(s: &State, k: &Kit) -> El {
    let mut top = row()
        .w_full()
        .gap(SM)
        .center_y()
        .child(wordmark(k).grow(1.0));
    let mut head = col().w_full().px(LG).py(MD).gap(SM);
    if !s.has_tunnels() {
        return head.child(top);
    }
    if !s.current_data() {
        return head.child(top.child(text("Unknown").font(HINT).color(k.c.text_muted)));
    }
    let (active, total) = s.active_count();
    top = top.child(
        text(format!("{active} of {total} active"))
            .font(HINT)
            .color(k.c.text_muted)
            .key("panel-active"),
    );
    let failed = s
        .model
        .tunnels
        .iter()
        .filter(|t| t.enabled && t.state.failure())
        .count();
    if failed > 0 {
        top = top.child(indicator(Tone::Danger, &format!("{failed} failed"), k));
    }
    head = head.child(top);
    let series = s.model.history.series(None, traffic_graph::now(s));
    let peak = series
        .iter()
        .map(|p| p.bytes_in.max(p.bytes_out))
        .max()
        .unwrap_or(0);
    let latest = series.last().copied().unwrap_or_default();
    let size = [s.graph_width.max(1.0), LG];
    let mut shape = Shape::new().stroke(vec![[0.0, LG], [size[0], LG]], RULE, k.c.border_base);
    if peak > 0 {
        shape = shape
            .stroke(
                traffic_graph::points(&series, peak, size, false),
                RULE,
                k.c.info.fg,
            )
            .stroke(
                traffic_graph::points(&series, peak, size, true),
                RULE,
                k.c.success.fg,
            );
    }
    head.child(
        row()
            .w_full()
            .gap(MD)
            .center_y()
            .child(indicator(
                Tone::Info,
                &format!("In  {}/s", traffic_graph::bytes(latest.bytes_in)),
                k,
            ))
            .child(indicator(
                Tone::Success,
                &format!("Out  {}/s", traffic_graph::bytes(latest.bytes_out)),
                k,
            ))
            .child(
                rune::canvas(shape)
                    .grow(1.0)
                    .min_w(0.0)
                    .h(LG)
                    .role("img")
                    .label("Combined traffic over the last 15 minutes")
                    .key("panel-traffic")
                    .on_layout(|s: &mut State, [_, _, width, _]| s.graph_width = width),
            ),
    )
}

impl State {
    /// Active tunnels and every tunnel, saved and terminal, as the summaries count them.
    pub(super) fn active_count(&self) -> (usize, usize) {
        let active = self
            .model
            .tunnels
            .iter()
            .map(|t| t.state)
            .chain(self.model.foreground.iter().map(|t| t.state))
            .filter(|state| *state == TunnelState::Active)
            .count();
        (
            active,
            self.model.tunnels.len() + self.model.foreground.len(),
        )
    }
}
