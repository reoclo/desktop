use super::{State, primitives::*};
use crate::theme::*;
use rune::{El, Live, ScrollPos, ScrollSpec, col, el, row, text};
use rune_kit::{Button, Kit, SortDir, Table, TableColumn, TableStyle};

const HEADER_HEIGHT: f32 = 32.0;
const ROW_HEIGHT: f32 = 36.0;
/// Rows the table keeps when the page is too short for more; the page scrolls instead.
const MIN_VISIBLE_ROWS: usize = 4;

pub fn view(s: &State, k: &Kit) -> El {
    let (active, saved) = s.active_count();
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
    let body = col()
        .w_full()
        .gap(LG)
        .child(
            body.on_layout(|s: &mut State, [_, _, _, height]| s.tunnels_space[1] = height)
                .key("tunnels-above"),
        )
        .child(if s.tunnel_row_count() > 0 {
            table(s, k)
        } else {
            text("No tunnels yet. Add a tunnel to reach a service.")
                .font(BODY)
                .color(k.c.text_muted)
                .wrap()
        });
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
                    .on_layout(|s: &mut State, [_, _, _, height]| s.tunnels_space[0] = height)
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
    let mut columns = if narrow {
        vec![TableColumn::new("compact", "Tunnel").flex(1.0).min(0.0)]
    } else {
        vec![
            TableColumn::new("state", "State")
                .width(STATE_WIDTH)
                .sortable(),
            TableColumn::new("name", "Name")
                .width(name_width)
                .sortable(),
            TableColumn::new("server", "Server")
                .width(server_width)
                .sortable(),
            TableColumn::new("endpoint", "Endpoint")
                .flex(1.0)
                .min(ENDPOINT_MIN),
        ]
    };
    columns.push(TableColumn::new("on", "On / Stop").width(ACTION_WIDTH));
    let style = TableStyle {
        selected: k.c.layer_03,
        row_height,
        header_height: HEADER_HEIGHT,
        header_font: LABEL,
        ..TableStyle::from_kit(k)
    };
    // The column header stays above the rows, which scroll on their own below it.
    let header = columns
        .iter()
        .cloned()
        .fold(
            Table::new(
                |s: &State| &s.table,
                |s: &mut State| &mut s.table,
                |_: &State| 0,
                super::tunnel_rows::key,
            ),
            Table::column,
        )
        .style(style)
        .on_sort(sort)
        .build(s, k)
        .focusable(false)
        .h(HEADER_HEIGHT)
        .w_full()
        .key("tunnels-columns");
    let rows = columns
        .into_iter()
        .fold(
            Table::new(
                |s: &State| &s.table,
                |s: &mut State| &mut s.table,
                State::tunnel_row_count,
                super::tunnel_rows::key,
            ),
            Table::column,
        )
        .name("Tunnels")
        .style(style)
        .cell(move |s: &State, i, column| {
            if column == "compact" {
                super::compact_tunnels::cell(s, i, &cell_kit)
            } else {
                super::tunnel_rows::cell(s, i, column, &cell_kit)
            }
        })
        .on_select(|s: &mut State, name, tx| s.reveal_tunnel(name, tx))
        .on_sort(sort)
        .build(s, k);
    let content = f32::from(u16::try_from(s.tunnel_row_count()).unwrap_or(u16::MAX)) * row_height;
    let shown = table_height(s, row_height) - HEADER_HEIGHT;
    let overflow = content > shown;
    let thumb = Live::new([0.0; 4]);
    let follow = thumb.clone();
    let mut viewport = col()
        .key("tunnel-rows-scroll")
        .w_full()
        .h(shown)
        .scroll(ScrollSpec {
            stick_bottom: false,
        })
        .scroll_margin(LG)
        .on_scroll_offset(move |at| {
            follow.set(thumb_rect(at));
        })
        // The rows' own copy of the header sits above the scroll's top, out of sight.
        .child(
            col().w_full().h(content).child(
                rows.h(HEADER_HEIGHT + content)
                    .mt(-HEADER_HEIGHT)
                    .w_full()
                    .key("table"),
            ),
        );
    let mut area = col().key("tunnel-rows").w_full().h(shown);
    if overflow {
        viewport = viewport.scroll_edge_fade(XL);
        area = area.child(viewport).child(
            el().key("tunnels-scroll-thumb")
                .absolute(0.0, 0.0)
                .live_rect(&thumb)
                .radius(XS / 2.0)
                .fill(k.c.border_strong)
                .a11y_hidden(),
        );
    } else {
        area = area.child(viewport);
    }
    col()
        .key("saved-tunnels")
        .w_full()
        .child(header)
        .child(area)
}

fn sort(s: &mut State, column: &str, direction: SortDir, _: &mut rune::Tx) {
    s.model.tunnels.sort_by(|a, b| {
        direction.apply(match column {
            "state" => a.state.label().cmp(b.state.label()),
            "server" => a.place().cmp(b.place()),
            _ => a.name.cmp(&b.name),
        })
    });
}

/// The scroll thumb, shown at rest while the rows overflow: as long as the share of rows in view.
fn thumb_rect(at: ScrollPos) -> [f32; 4] {
    let [width, height] = at.viewport;
    let max = at.max[1];
    if max <= 0.0 {
        return [0.0; 4];
    }
    let track = height - XS * 2.0;
    let length = (track * height / (height + max)).max(XL);
    let top = XS + (track - length) * (at.offset[1] / max).clamp(0.0, 1.0);
    [width - XS * 2.0, top, XS, length]
}

/// Every row when they fit below the details and graph, otherwise as many whole rows as fit,
/// so the table fills the page and scrolls inside itself only when its rows exceed it.
fn table_height(s: &State, row_height: f32) -> f32 {
    let [page, above] = s.tunnels_space;
    let count = |rows: usize| f32::from(u16::try_from(rows).unwrap_or(u16::MAX));
    let rows = count(s.tunnel_row_count());
    let room = ((page - above - LG - HEADER_HEIGHT - RULE) / row_height).floor();
    let shown = if rows <= room {
        rows
    } else {
        room.max(count(MIN_VISIBLE_ROWS).min(rows))
    };
    HEADER_HEIGHT + shown * row_height + RULE
}
