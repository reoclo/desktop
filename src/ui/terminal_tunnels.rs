use super::{State, primitives::*};
use crate::{model::ForegroundForward, theme::*};
use rune::{El, col, row, text};
use rune_kit::{Button, ControlSize, Kit};

fn forward_line(f: &ForegroundForward) -> String {
    let line = format!("{} \u{2192} {}", f.listen, f.target);
    if f.proto == "udp" {
        format!("{line} (udp)")
    } else {
        line
    }
}

fn ask_to_stop(s: &mut State, id: &str) {
    s.stop_target = Some(id.to_owned());
    s.modal = Some("stop-terminal".into());
}

/// Tunnels started with `reoclo tunnel` in a terminal, reported by the daemon.
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
        for (index, forward) in tunnel.forwards.iter().enumerate() {
            let line = forward_line(forward);
            let mut entry = row().w_full().gap(MD).center_y().min_w(0.0).child(
                text(&line)
                    .font(MONO)
                    .color(k.c.text_muted)
                    .truncate()
                    .selectable(true)
                    .help_when_truncated(&line)
                    .min_w(0.0)
                    .shrink(),
            );
            if !s.panel && forward.kind == "forward" && forward.proto == "tcp" {
                let org = tunnel.org.clone();
                let server = tunnel.server.clone();
                let target = forward.target.clone();
                let name = format!("web-{}", target.rsplit(':').next().unwrap_or("tunnel"));
                entry = entry.child(
                    Button::new("Add HTTPS")
                        .control_size(ControlSize::Small)
                        .help("Reach this service at a local HTTPS address")
                        .on_press(move |s: &mut State| {
                            s.begin_https(&org, &server, &target, name.clone())
                        })
                        .build(k)
                        .border(RULE, k.c.border_strong)
                        .key(format!("https-terminal-{}-{index}", tunnel.id)),
                );
            }
            forwards = forwards.child(entry);
        }
        let id = tunnel.id.clone();
        let key = format!("stop-terminal-{}", tunnel.id);
        if s.compact {
            let expanded = s.expanded_terminal.contains(&tunnel.id);
            let expand_id = tunnel.id.clone();
            let mut item = col().w_full().gap(SM).child(
                row()
                    .w_full()
                    .gap(SM)
                    .center_y()
                    .child(
                        Button::new(format!("{} ({})", tunnel.server, tunnel.forwards.len()))
                            .help("Show or hide the forwarded ports")
                            .on_press(move |s: &mut State| {
                                if !s.expanded_terminal.remove(&expand_id) {
                                    s.expanded_terminal.insert(expand_id.clone());
                                }
                            })
                            .secondary(k)
                            .key(format!("expand-terminal-{}", tunnel.id)),
                    )
                    .child(spacer())
                    .child(status(tunnel.state, k))
                    .child(
                        Button::new("Stop")
                            .on_press(move |s: &mut State| ask_to_stop(s, &id))
                            .danger(k)
                            .key(key),
                    ),
            );
            if expanded {
                item = item.child(forwards);
            }
            section = section.child(item);
            continue;
        }
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
        section = section.child(if s.panel {
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
                                .on_press(move |s: &mut State| ask_to_stop(s, &id))
                                .danger(k)
                                .key(key),
                        ),
                )
                .child(forwards)
        } else {
            row()
                .w_full()
                .gap(LG)
                .center_y()
                .child(
                    col()
                        .grow(1.0)
                        .min_w(0.0)
                        .gap(XS)
                        .child(name)
                        .child(forwards),
                )
                .child(status(tunnel.state, k))
                .child(
                    Button::new("Stop")
                        .disabled(s.busy)
                        .on_press(move |s: &mut State| ask_to_stop(s, &id))
                        .danger(k)
                        .key(key),
                )
        });
    }
    Some(section)
}
