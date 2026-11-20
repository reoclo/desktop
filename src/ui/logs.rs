use super::{Page, State, primitives::*};
use crate::{logs::LogEntry, theme::*};
use rune::{El, GrowthSpec, ScrollSpec, TextEdit, col, row, text};
use rune_kit::{Button, Kit, TextInput};
use std::{borrow::Cow, rc::Rc};

pub struct LogView {
    pub query: TextEdit,
    pub tunnel: TextEdit,
    pub paused: Option<Vec<LogEntry>>,
    pub follow: bool,
}

impl Default for LogView {
    fn default() -> Self {
        Self {
            query: TextEdit::new(""),
            tunnel: TextEdit::new(""),
            paused: None,
            follow: true,
        }
    }
}

impl State {
    pub fn show_logs(&mut self, tunnel: &str) {
        self.show_page(Page::Logs);
        self.logs.tunnel = TextEdit::new(tunnel);
        self.logs.query = TextEdit::new("");
        self.logs.paused = None;
        self.logs.follow = true;
    }

    pub fn filtered_logs(&self) -> Vec<LogEntry> {
        let source: Box<dyn Iterator<Item = &LogEntry> + '_> = match &self.logs.paused {
            Some(entries) => Box::new(entries.iter()),
            None => Box::new(self.model.recent_logs.iter()),
        };
        source
            .filter(|entry| {
                entry.matches(self.logs.tunnel.text.trim(), self.logs.query.text.trim())
            })
            .cloned()
            .collect()
    }
}

pub fn view(s: &State, k: &Kit) -> El {
    let entries = Rc::new(s.filtered_logs());
    let count = entries.len();
    let keys = Rc::clone(&entries);
    let color = k.c.text_base;
    let selection = k.selection();
    let copy = entries
        .iter()
        .map(LogEntry::display)
        .collect::<Vec<_>>()
        .join("\n");
    let filters = col()
        .equal_columns(if s.compact { 1 } else { 2 })
        .w_full()
        .gap(SM)
        .child(
            TextInput::new(&s.logs.tunnel)
                .label("Filter by tunnel")
                .placeholder("All tunnels")
                .on_edit(|s: &mut State| &mut s.logs.tunnel)
                .build(k)
                .w_full()
                .border(RULE, k.c.border_strong)
                .key("log-tunnel-filter"),
        )
        .child(
            TextInput::new(&s.logs.query)
                .label("Filter log text")
                .placeholder("Search log text")
                .on_edit(|s: &mut State| &mut s.logs.query)
                .build(k)
                .w_full()
                .border(RULE, k.c.border_strong)
                .key("log-text-filter"),
        );
    let mut controls = (if s.viewport_width < NARROW_WIDTH {
        col().equal_columns(2)
    } else {
        row().flex_wrap()
    })
    .w_full()
    .gap(SM)
    .child(
        Button::new(if s.logs.paused.is_some() {
            "Resume"
        } else {
            "Pause"
        })
        .on_press(|s: &mut State| {
            s.logs.paused = match s.logs.paused {
                Some(_) => None,
                None => Some(s.model.recent_logs.iter().cloned().collect()),
            };
        })
        .secondary(k)
        .key("pause-logs"),
    )
    .child(
        Button::new(if s.logs.follow { "Following" } else { "Follow" })
            .on_press_tx(|s: &mut State, tx| {
                s.logs.follow = !s.logs.follow;
                if s.logs.follow {
                    tx.scroll_to_bottom_instant("logs-scroll");
                }
            })
            .secondary(k)
            .key("follow-logs"),
    )
    .child(
        Button::new("Copy")
            .disabled(count == 0)
            .on_press_tx(move |s: &mut State, tx| {
                tx.copy(&copy);
                s.notice = Some("Filtered log copied.".into());
            })
            .secondary(k)
            .key("copy-events"),
    );
    if let Some(root) = s.socket_path.parent().and_then(std::path::Path::parent) {
        let path = root.join("tunneld.log");
        controls = controls.child(
            Button::new("Show log in Finder")
                .on_press(move |s: &mut State| {
                    s.run_system(crate::system::SystemAction::ShowLog(path.clone()));
                })
                .secondary(k)
                .key("show-daemon-log"),
        );
    }
    let mut body = col()
        .w_full()
        .max_w(CONTENT_MAX)
        .h_full()
        .padding(if s.compact { LG } else { XL })
        .gap(LG)
        .child(title(
            "Logs",
            Some("Daemon events and tunneld.log. Timestamps are UTC."),
            k,
        ))
        .child(filters)
        .child(controls);
    if let Some(error) = &s.model.log_error {
        body = body.child(alert(&format!("Could not read tunneld.log: {error}"), k));
    }
    let rows = col()
        .key("log-rows")
        .w_full()
        .growth(GrowthSpec::default())
        .virtual_rows(
            count,
            LOG_ROW_HEIGHT,
            move |_: &State, i| Cow::Owned(format!("log-entry-{}", keys[i].id)),
            move |_: &State, i| {
                let line = entries[i].display();
                text(&line)
                    .font(MONO)
                    .color(color)
                    .h(LOG_ROW_HEIGHT)
                    .w_full()
                    .truncate()
                    .help_when_truncated(line)
                    .selectable(true)
                    .selection_color(selection)
            },
        );
    body.child(
        text(format!(
            "{count} {}{}",
            if count == 1 { "line" } else { "lines" },
            if s.logs.paused.is_some() {
                " · Paused"
            } else {
                " · Live"
            }
        ))
        .role("status")
        .font(HINT)
        .color(k.c.text_muted),
    )
    .child(
        k.scroll_bar(
            col()
                .key("logs-scroll")
                .w_full()
                .grow(1.0)
                .min_h(0.0)
                .border(RULE, k.c.border_base)
                .padding(SM)
                .scroll(ScrollSpec {
                    stick_bottom: s.logs.follow && s.logs.paused.is_none(),
                })
                .child(if count == 0 {
                    text("No matching log lines.")
                        .font(MONO)
                        .color(k.c.text_muted)
                        .padding(SM)
                } else {
                    rows
                }),
        ),
    )
}
