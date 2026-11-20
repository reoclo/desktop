use super::{State, primitives::*};
use crate::{
    control::segment,
    model::{ForegroundTunnel, TunnelState, TunnelView},
    theme::*,
};
use rune::{col, row, text};
use rune_kit::{Button, Cell, ControlSize, Kit, Switch};
use std::borrow::Cow;

pub(super) enum TunnelRow<'a> {
    Saved(&'a TunnelView),
    Terminal(&'a ForegroundTunnel, usize),
}

impl State {
    pub(super) fn tunnel_row_count(&self) -> usize {
        self.model.tunnels.len()
            + self
                .model
                .foreground
                .iter()
                .map(|t| t.forwards.len().max(1))
                .sum::<usize>()
    }

    pub(super) fn tunnel_row(&self, index: usize) -> Option<TunnelRow<'_>> {
        if let Some(tunnel) = self.model.tunnels.get(index) {
            return Some(TunnelRow::Saved(tunnel));
        }
        self.model
            .foreground
            .iter()
            .flat_map(|t| (0..t.forwards.len().max(1)).map(move |i| TunnelRow::Terminal(t, i)))
            .nth(index.checked_sub(self.model.tunnels.len())?)
    }
}

pub(super) fn key(s: &State, index: usize) -> Cow<'_, str> {
    match s.tunnel_row(index) {
        Some(TunnelRow::Saved(t)) => Cow::Borrowed(&t.name),
        Some(TunnelRow::Terminal(t, i)) => Cow::Owned(format!("terminal:{}:{i}", t.id)),
        None => Cow::Owned(format!("missing-{index}")),
    }
}

pub(super) fn cell(s: &State, index: usize, column: &str, k: &Kit) -> Cell {
    match s.tunnel_row(index) {
        Some(TunnelRow::Saved(t)) => saved(t, column, s, k),
        Some(TunnelRow::Terminal(t, i)) => terminal(t, i, column, s, k),
        None => Cell::el(rune::el()),
    }
}

fn saved(t: &TunnelView, column: &str, s: &State, k: &Kit) -> Cell {
    match column {
        "state" => Cell::el(if s.current_data() && t.retry.is_some() {
            status(TunnelState::Reconnecting, k)
        } else {
            status(
                if s.current_data() {
                    t.state
                } else {
                    TunnelState::Unknown
                },
                k,
            )
        }),
        "name" => Cell::el(
            text(&t.name)
                .font(BODY)
                .color(k.c.text_base)
                .truncate()
                .help_when_truncated(&t.name)
                .min_w(0.0),
        ),
        "server" => Cell::el(
            text(t.place())
                .font(if t.place() == "This Mac" { HINT } else { MONO })
                .color(k.c.text_muted)
                .truncate_middle()
                .help_when_truncated(t.place())
                .min_w(0.0),
        ),
        "on" if s.current_data() => {
            let path = format!(
                "/v1/tunnels/{}/{}",
                segment(&t.name),
                if t.enabled { "down" } else { "up" }
            );
            Cell::el(
                Switch::new(t.enabled)
                    .label(format!("Run {}", t.name))
                    .hide_label()
                    .disabled(s.tunnel_pending(&t.name))
                    .on_toggle(move |s: &mut State| s.request("POST", &path, None))
                    .build(k)
                    .key(format!("toggle-row-{}", t.name)),
            )
        }
        "on" => Cell::el(rune::el()),
        _ => Cell::el(
            text(t.endpoint())
                .font(MONO)
                .color(if t.state.available() && s.current_data() {
                    k.c.text_base
                } else {
                    k.c.text_muted
                })
                .truncate()
                .help_when_truncated(t.endpoint())
                .min_w(0.0),
        ),
    }
}

fn terminal(t: &ForegroundTunnel, index: usize, column: &str, s: &State, k: &Kit) -> Cell {
    match column {
        "state" => Cell::el(status(
            if s.current_data() {
                t.state
            } else {
                TunnelState::Unknown
            },
            k,
        )),
        "name" => Cell::el(
            text("Terminal")
                .font(HINT)
                .color(k.c.text_muted)
                .help(format!("PID {} · {}", t.pid, t.org)),
        ),
        "server" => Cell::el(
            text(&t.server)
                .font(MONO)
                .color(k.c.text_base)
                .truncate_middle()
                .help_when_truncated(&t.server)
                .min_w(0.0),
        ),
        "on" if s.current_data() => {
            let id = t.id.clone();
            Cell::el(
                Button::new("Stop")
                    .control_size(ControlSize::Small)
                    .on_press(move |s: &mut State| {
                        s.stop_target = Some(id.clone());
                        s.modal = Some("stop-terminal".into());
                    })
                    .variant(rune_kit::ButtonVariant::Danger)
                    .build(k)
                    .border(RULE, k.c.danger.fg)
                    .key(if index == 0 {
                        format!("stop-terminal-{}", t.id)
                    } else {
                        format!("stop-terminal-{}-{index}", t.id)
                    }),
            )
        }
        "on" => Cell::el(rune::el()),
        _ => {
            let Some(f) = t.forwards.get(index) else {
                return Cell::text("No forwards");
            };
            let line = format!("{} → {}", f.listen, f.target);
            let mut content = row().w_full().min_w(0.0).gap(SM).center_y().child(
                text(&line)
                    .font(MONO)
                    .color(k.c.text_base)
                    .truncate()
                    .help_when_truncated(&line)
                    .min_w(0.0)
                    .shrink(),
            );
            if let Some(action) = https_button(s, (t, index), k) {
                content = content.child(action);
            }
            Cell::el(content)
        }
    }
}

pub(super) fn https_button(
    s: &State,
    (t, index): (&ForegroundTunnel, usize),
    k: &Kit,
) -> Option<rune::El> {
    let f = t.forwards.get(index)?;
    if !s.current_data() || f.kind != "forward" || f.proto != "tcp" {
        return None;
    }
    let (org, server, target) = (t.org.clone(), t.server.clone(), f.target.clone());
    Some(
        Button::new("Add HTTPS")
            .control_size(ControlSize::Small)
            .on_press(move |s: &mut State| {
                let name = format!("web-{}", target.rsplit(':').next().unwrap_or("tunnel"));
                s.begin_https(&org, &server, &target, name);
            })
            .build(k)
            .border(RULE, k.c.border_strong)
            .key(format!("https-terminal-{}-{index}", t.id)),
    )
}

pub(super) fn terminal_detail(s: &State, k: &Kit) -> Option<rune::El> {
    let selected = s.model.selected.as_deref()?;
    let row = (s.model.tunnels.len()..s.tunnel_row_count()).find(|i| key(s, *i) == selected)?;
    let TunnelRow::Terminal(t, _) = s.tunnel_row(row)? else {
        return None;
    };
    Some(
        col()
            .w_full()
            .gap(SM)
            .child(
                col()
                    .gap(XS)
                    .key("tunnel-header")
                    .child(
                        text(&t.server)
                            .role("heading")
                            .font(HEADING)
                            .color(k.c.text_base),
                    )
                    .child(text("Terminal tunnel").font(HINT).color(k.c.text_muted)),
            )
            .child(
                text(if t.traffic.is_some() {
                    "Traffic includes every forward in this terminal session."
                } else {
                    "Restart this terminal tunnel with CLI 0.116.0 to collect traffic."
                })
                .font(HINT)
                .color(k.c.text_muted)
                .wrap(),
            ),
    )
}
