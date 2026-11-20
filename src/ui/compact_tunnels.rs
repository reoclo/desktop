use super::{
    State,
    primitives::status,
    tunnel_rows::{TunnelRow, https_button},
};
use crate::{model::TunnelState, theme::*};
use rune::{col, row, text};
use rune_kit::{Cell, Kit};

pub(super) fn cell(s: &State, index: usize, k: &Kit) -> Cell {
    let Some(tunnel) = s.tunnel_row(index) else {
        return Cell::el(rune::el());
    };
    let (name, state, endpoint, action) = match tunnel {
        TunnelRow::Saved(t) => (
            t.name.clone(),
            if t.retry.is_some() {
                TunnelState::Reconnecting
            } else {
                t.state
            },
            t.endpoint().to_owned(),
            None,
        ),
        TunnelRow::Terminal(t, i) => (
            format!("Terminal · {}", t.server),
            t.state,
            t.forwards.get(i).map_or_else(
                || "No forwards".into(),
                |f| format!("{} → {}", f.listen, f.target),
            ),
            https_button(s, (t, i), k),
        ),
    };
    let mut address = row().w_full().gap(SM).center_y().min_w(0.0).child(
        text(&endpoint)
            .font(MONO)
            .color(k.c.text_muted)
            .truncate_middle()
            .help_when_truncated(endpoint)
            .min_w(0.0)
            .grow(1.0),
    );
    if let Some(action) = action {
        address = address.child(action);
    }
    Cell::el(
        col()
            .w_full()
            .gap(XS)
            .child(
                text(&name)
                    .font(BODY)
                    .color(k.c.text_base)
                    .truncate()
                    .help_when_truncated(name)
                    .min_w(0.0),
            )
            .child(row().w_full().gap(SM).child(status(
                if s.current_data() {
                    state
                } else {
                    TunnelState::Unknown
                },
                k,
            )))
            .child(address),
    )
}
