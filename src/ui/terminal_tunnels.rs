use super::{State, primitives::*};
use crate::theme::*;
use rune::{El, col, row, text};
use rune_kit::{Button, ControlSize, Kit};

/// The menu-bar panel's terminal sessions. The main window uses the unified table.
pub fn terminal_tunnels(s: &State, k: &Kit) -> Option<El> {
    if s.model.foreground.is_empty() {
        return None;
    }
    let mut section = col().key("terminal-tunnels").w_full().gap(MD).child(
        text(format!(
            "Running in a terminal ({})",
            s.model.foreground.len()
        ))
        .role("heading")
        .font(LABEL)
        .color(k.c.text_base),
    );
    for tunnel in &s.model.foreground {
        let mut forwards = col().w_full().min_w(0.0).gap(XS);
        for forward in &tunnel.forwards {
            let line = format!(
                "{} → {}{}",
                forward.listen,
                forward.target,
                if forward.proto == "udp" { " (udp)" } else { "" }
            );
            forwards = forwards.child(
                row().w_full().gap(MD).center_y().min_w(0.0).child(
                    text(&line)
                        .font(MONO)
                        .color(k.c.text_muted)
                        .truncate()
                        .selectable(true)
                        .help_when_truncated(&line)
                        .min_w(0.0)
                        .shrink(),
                ),
            );
        }
        let id = tunnel.id.clone();
        let name = row()
            .gap(SM)
            .center_y()
            .min_w(0.0)
            .shrink()
            .child(
                text(&tunnel.server)
                    .font(BODY)
                    .color(k.c.text_base)
                    .truncate()
                    .help_when_truncated(&tunnel.server)
                    .min_w(0.0)
                    .shrink(),
            )
            .child(text(&tunnel.org).font(HINT).color(k.c.text_muted));
        section = section.child(
            col()
                .w_full()
                .gap(XS)
                .child(
                    row()
                        .w_full()
                        .gap(SM)
                        .center_y()
                        .child(name.grow(1.0))
                        .child(status(tunnel.state, k))
                        .child(
                            Button::new("Stop")
                                .control_size(ControlSize::Small)
                                .help(format!("Stop {}", tunnel.server))
                                .on_press(move |s: &mut State| {
                                    s.stop_target = Some(id.clone());
                                    s.modal = Some("stop-terminal".into());
                                })
                                .danger(k)
                                .key(format!("stop-terminal-{}", tunnel.id)),
                        ),
                )
                .child(forwards),
        );
    }
    Some(section)
}
