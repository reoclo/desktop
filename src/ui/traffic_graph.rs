use super::{State, primitives::*};
use crate::{theme::*, traffic::Point};
use rune::{El, Shape, col, row, text};
use rune_kit::{Button, Kit};

/// The chart uses fixed one-second buckets, with no interpolation or looping animation.
pub fn view(s: &State, k: &Kit) -> El {
    let now = (s.clock_epoch + time::Duration::seconds_f64(s.clock_seconds.unwrap_or(0.0)))
        .unix_timestamp();
    let selected = s.model.selected.as_deref().map(|name| {
        if name.starts_with("terminal:") {
            name.rsplit_once(':').map_or(name, |(session, _)| session)
        } else {
            name
        }
    });
    let series = s.model.history.series(selected, now);
    let peak = series
        .iter()
        .map(|p| p.bytes_in.max(p.bytes_out))
        .max()
        .unwrap_or(0);
    let latest = series.last().copied().unwrap_or_default();
    let mut heading = row()
        .w_full()
        .gap(LG)
        .center_y()
        .flex_wrap()
        .child(
            text(if selected.is_some() {
                "Traffic"
            } else {
                "All tunnels"
            })
            .role("heading")
            .font(HEADING)
            .color(k.c.text_base),
        )
        .child(indicator(
            Tone::Info,
            &format!("In  {}/s", bytes(latest.bytes_in)),
            k,
        ))
        .child(indicator(
            Tone::Success,
            &format!("Out  {}/s", bytes(latest.bytes_out)),
            k,
        ))
        .child(spacer());
    if selected.is_some() {
        heading = heading.child(
            Button::new("All tunnels")
                .on_press(|s: &mut State| {
                    s.model.selected = None;
                    s.table.clear_selection();
                })
                .secondary(k)
                .key("all-tunnels"),
        );
    }
    let width = s.graph_width.max(1.0);
    let height = GRAPH_HEIGHT;
    let mut shape = Shape::new()
        .stroke(vec![[0.0, height], [width, height]], RULE, k.c.border_base)
        .stroke(vec![[0.0, 0.0], [width, 0.0]], RULE, k.c.border_base);
    if peak > 0 {
        shape = shape
            .stroke(
                points(&series, peak, width, false),
                GRAPH_STROKE,
                k.c.info.fg,
            )
            .stroke(
                points(&series, peak, width, true),
                GRAPH_STROKE,
                k.c.success.fg,
            );
    }
    let mut graph = col()
        .w_full()
        .gap(SM)
        .key("traffic-graph")
        .on_layout(|s: &mut State, [_, _, width, _]| s.graph_width = width)
        .child(heading);
    if peak == 0 {
        graph = graph.child(text("No traffic yet").font(HINT).color(k.c.text_muted));
    } else {
        graph = graph.child(
            text(format!("Peak {}/s", bytes(peak)))
                .font(HINT)
                .color(k.c.text_muted),
        );
    }
    graph.child(rune::canvas(shape).w_full().h(height).role("img")
        .label(format!("Traffic over the last 15 minutes. In {} bytes per second. Out {} bytes per second.",
            latest.bytes_in, latest.bytes_out)))
        .child(row().w_full().child(text("15 min ago").font(MONO).color(k.c.text_muted))
            .child(spacer()).child(text("Now").font(MONO).color(k.c.text_muted)))
}

fn points(series: &[Point], peak: u64, width: f32, outgoing: bool) -> Vec<[f32; 2]> {
    // Five-second maxima keep short bursts visible without drawing more vertices than pixels.
    let buckets: Vec<_> = series
        .chunks(5)
        .map(|chunk| {
            chunk
                .iter()
                .map(|p| if outgoing { p.bytes_out } else { p.bytes_in })
                .max()
                .unwrap_or(0)
        })
        .collect();
    let last = u16::try_from(buckets.len().saturating_sub(1))
        .unwrap_or(1)
        .max(1);
    buckets
        .iter()
        .enumerate()
        .map(|(i, value)| {
            let x = f32::from(u16::try_from(i).unwrap_or(last)) / f32::from(last) * width;
            let scaled = value.saturating_mul(10_000).checked_div(peak).unwrap_or(0);
            let ratio = f32::from(u16::try_from(scaled).unwrap_or(10_000)) / 10_000.0;
            [x, GRAPH_HEIGHT * (1.0 - ratio)]
        })
        .collect()
}

fn bytes(value: u64) -> String {
    if value >= 1024 * 1024 {
        format!("{} MiB", value / (1024 * 1024))
    } else if value >= 1024 {
        format!("{} KiB", value / 1024)
    } else {
        format!("{value} B")
    }
}
