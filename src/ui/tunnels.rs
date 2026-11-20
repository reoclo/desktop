use super::{State, primitives::*};
use crate::{model::TunnelState, theme::*};
use rune::{El, ScrollSpec, col, row, text};
use rune_kit::{Button, Kit, Table, TableColumn, TableStyle};

const HEADER_HEIGHT: f32 = 32.0;
const ROW_HEIGHT: f32 = 36.0;
const MAX_VISIBLE_ROWS: usize = 8;

pub fn view(s: &State, k: &Kit) -> El {
    let saved = s.model.tunnels.len() + s.model.foreground.len();
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
    let failed = s.model.tunnels.iter().filter(|t| t.state.failure()).count();
    let mut summary = row().gap(LG).center_y().child(
        text(if s.current_data() {
            format!("{active} of {saved} active")
        } else {
            format!("{saved} saved, last known configuration")
        })
        .font(BODY)
        .color(k.c.text_muted),
    );
    if failed > 0 && s.current_data() {
        summary = summary.child(indicator(Tone::Danger, &format!("{failed} failed"), k));
    }
    let mut header = col().w_full().gap(LG).child(
        row()
            .w_full()
            .center_y()
            .gap(LG)
            .child(
                col()
                    .gap(XS)
                    .grow(1.0)
                    .child(title("Tunnels", None, k))
                    .child(summary),
            )
            .child(
                Button::new("Add tunnel")
                    .shortcut("cmd+n")
                    .on_press(|s: &mut State| s.begin_add())
                    .primary(k)
                    .key("add-tunnel"),
            ),
    );
    if !s.current_data() {
        header = header.child(super::data_state::phase_alert(s, k));
    } else {
        if let Some(error) = s.error.as_ref().filter(|_| s.modal.is_none()) {
            header = header.child(alert(error, k));
        }
        if !s.shared_failures().is_empty() {
            header = header.child(super::tunnel_health::shared_alerts(s, k));
        }
    }
    let mut body = col().w_full().gap(LG);
    if let Some(tunnel) = s.selected() {
        body = body.child(super::tunnel_detail::view(tunnel, s, k));
    } else if let Some(detail) = super::tunnel_rows::terminal_detail(s, k) {
        body = body.child(detail);
    }
    body = body.child(super::traffic_graph::view(s, k));
    if s.current_data()
        && let Some(requests) = approvals(s, k)
    {
        body = body.child(
            requests
                .padding(MD)
                .border(RULE, k.c.border_base)
                .radius(k.radius.xl),
        );
    }
    if s.tunnel_row_count() > 0 {
        body = body.child(table(s, k));
    } else {
        body = body.child(
            text("No tunnels yet. Add a tunnel to reach a service.")
                .font(BODY)
                .color(k.c.text_muted)
                .wrap(),
        );
    }
    col()
        .key("tunnels-page")
        .w_full()
        .max_w(CONTENT_MAX)
        .h_full()
        .padding(if s.compact { LG } else { XL })
        .gap(LG)
        .child(header.pr(LG).key("tunnels-header"))
        .child(
            k.scroll_bar(
                col()
                    .key("tunnels-scroll")
                    .w_full()
                    .grow(1.0)
                    .min_h(0.0)
                    .scroll(ScrollSpec {
                        stick_bottom: false,
                    })
                    .child(body.pr(LG)),
            ),
        )
}

fn table(s: &State, k: &Kit) -> El {
    let measure = |value: &str| value.chars().fold(0.0, |width, _| width + MONO_ADVANCE) + XL;
    let wanted_name = s
        .model
        .tunnels
        .iter()
        .map(|t| measure(&t.name))
        .fold(COLUMN_MIN, f32::max)
        .clamp(COLUMN_MIN, COLUMN_MAX);
    let wanted_server = s
        .model
        .tunnels
        .iter()
        .map(|t| measure(t.place()))
        .fold(COLUMN_MIN, f32::max)
        .clamp(COLUMN_MIN, COLUMN_MAX);
    let padding = if s.compact { LG } else { XL };
    let content =
        (s.viewport_width - s.sidebar.shown_width()).min(CONTENT_MAX) - padding * 2.0 - LG;
    let labels = (content - STATE_WIDTH - ENDPOINT_MIN - ACTION_WIDTH).max(COLUMN_MIN * 2.0);
    let name_width = wanted_name.min((labels - wanted_server.min(labels / 2.0)).max(COLUMN_MIN));
    let server_width = wanted_server.min((labels - name_width).max(COLUMN_MIN));
    let cell_kit = k.clone();
    let narrow = s.viewport_width < NARROW_WIDTH;
    let row_height = if narrow {
        CONTROL * 2.0 + SM
    } else {
        ROW_HEIGHT
    };
    let mut table = Table::new(
        |s: &State| &s.table,
        |s: &mut State| &mut s.table,
        State::tunnel_row_count,
        super::tunnel_rows::key,
    )
    .name("Tunnels")
    .style(TableStyle {
        selected: k.c.layer_03,
        row_height,
        header_height: HEADER_HEIGHT,
        header_font: LABEL,
        ..TableStyle::from_kit(k)
    });
    if narrow {
        table = table.column(TableColumn::new("compact", "Tunnel").flex(1.0).min(0.0));
    } else {
        table = table
            .column(
                TableColumn::new("state", "State")
                    .width(STATE_WIDTH)
                    .sortable(),
            )
            .column(
                TableColumn::new("name", "Name")
                    .width(name_width)
                    .sortable(),
            )
            .column(
                TableColumn::new("server", "Server")
                    .width(server_width)
                    .sortable(),
            )
            .column(
                TableColumn::new("endpoint", "Endpoint")
                    .flex(1.0)
                    .min(ENDPOINT_MIN),
            );
    }
    table
        .column(TableColumn::new("on", "On / Stop").width(ACTION_WIDTH))
        .cell(move |s: &State, i, column| {
            if column == "compact" {
                super::compact_tunnels::cell(s, i, &cell_kit)
            } else {
                super::tunnel_rows::cell(s, i, column, &cell_kit)
            }
        })
        .on_select(|s: &mut State, name, tx| s.reveal_tunnel(name, tx))
        .on_sort(|s: &mut State, column, direction, _| {
            s.model.tunnels.sort_by(|a, b| {
                direction.apply(match column {
                    "state" => a.state.label().cmp(b.state.label()),
                    "server" => a.place().cmp(b.place()),
                    _ => a.name.cmp(&b.name),
                })
            });
        })
        .build(s, k)
        .h(HEADER_HEIGHT
            + u16::try_from(s.tunnel_row_count().clamp(1, MAX_VISIBLE_ROWS))
                .map(f32::from)
                .unwrap_or(8.0)
                * row_height
            + RULE)
        .w_full()
        .key("saved-tunnels")
}
