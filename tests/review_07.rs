use reoclo_desktop::ui::{State, view};
use rune::{ElementInfo, Env, Runtime};

fn runtime(state: State, size: (f32, f32)) -> Runtime<State> {
    let mut rt = Runtime::new(state, view, size, Env::new(false, 1.0));
    // The first frame measures the page; the next lay the table out in that space and let
    // its size settle on the virtual clock.
    rt.frame(0.0);
    rt.frame(0.02);
    rt.frame(1.0);
    rt
}

fn click(rt: &mut Runtime<State>, key: &str) {
    let (x, y) = rt.find(key).unwrap();
    rt.pointer_down(x, y);
    rt.pointer_up(x, y);
    rt.frame(0.02);
}

fn rect(rt: &Runtime<State>, key: &str) -> [f32; 4] {
    rt.presented(key).unwrap().0
}

fn element<'a>(elements: &'a [ElementInfo], key: &str) -> &'a ElementInfo {
    elements
        .iter()
        .find(|e| e.key.as_deref() == Some(key))
        .unwrap()
}

#[test]
fn the_panel_lists_terminal_forwards_as_ordinary_rows() {
    let rt = runtime(State::fixture("panel", "busy").unwrap(), (340.0, 620.0));
    let elements = rt.elements();
    let parent = element(&elements, "panel-row-grafana").parent;
    for key in ["panel-row-terminal:4242:0", "panel-row-terminal:4242:1"] {
        assert_eq!(element(&elements, key).parent, parent);
    }
    let texts: Vec<_> = elements.iter().filter_map(|e| e.text.as_deref()).collect();
    assert!(texts.contains(&"Terminal"));
    assert!(!texts.contains(&"This Mac"));
    assert!(!texts.iter().any(|t| t.starts_with("Running in a terminal")));
    assert!(rt.find("stop-terminal-4242").is_some());
}

#[test]
fn the_panel_puts_approvals_and_failures_before_other_tunnels() {
    let rt = runtime(State::fixture("panel", "busy").unwrap(), (340.0, 620.0));
    let approval = rect(&rt, "approve-approval-1")[1];
    let failed = rect(&rt, "panel-row-billing-api")[1];
    let first_saved = rect(&rt, "panel-row-grafana")[1];
    let terminal = rect(&rt, "panel-row-terminal:4242:0")[1];
    assert!(approval < failed, "{approval} < {failed}");
    assert!(failed < first_saved, "{failed} < {first_saved}");
    assert!(first_saved < terminal, "{first_saved} < {terminal}");
    assert_eq!(rt.state.model.tunnels.last().unwrap().name, "billing-api");
}

#[test]
fn the_panel_fits_twelve_rows_and_an_approval_without_scrolling() {
    let rt = runtime(State::fixture("panel", "busy").unwrap(), (340.0, 620.0));
    let viewport = rect(&rt, "panel-scroll");
    let last = rect(&rt, "panel-row-terminal:4242:1");
    assert!(
        last[1] + last[3] <= viewport[1] + viewport[3],
        "{last:?} inside {viewport:?}"
    );
    let rows = rt
        .elements()
        .iter()
        .filter(|e| {
            e.key
                .as_deref()
                .is_some_and(|k| k.starts_with("panel-row-"))
        })
        .count();
    assert_eq!(rows, 12);
}

#[test]
fn a_quiet_panel_list_takes_only_its_rows_so_the_window_can_fit_it() {
    let rt = runtime(State::fixture("panel", "quiet").unwrap(), (340.0, 620.0));
    let last = rect(&rt, "panel-row-terminal:4242:1");
    let quit = rect(&rt, "quit");
    assert!(quit[1] - (last[1] + last[3]) <= 24.0, "{last:?} {quit:?}");
    assert!(quit[1] + quit[3] < 400.0, "{quit:?}");
}

#[test]
fn terminal_tags_keep_a_gutter_before_the_endpoint() {
    let mut state = State::fixture("panel", "busy").unwrap();
    state.model.foreground[0].server = "bastion-production-eu-1".into();
    let rt = runtime(state, (340.0, 620.0));
    let elements = rt.elements();
    let tag = elements
        .iter()
        .find(|e| e.text.as_deref() == Some("Terminal"))
        .unwrap()
        .rect;
    let endpoint = rect(&rt, "copy-bastion-production-eu-1");
    assert!(tag[0] + tag[2] + 8.0 <= endpoint[0], "{tag:?} {endpoint:?}");
}

#[test]
fn the_table_takes_every_row_it_needs_when_the_page_has_room() {
    let rt = runtime(State::fixture("tunnels", "busy").unwrap(), (1920.0, 1080.0));
    let table = rect(&rt, "saved-tunnels");
    assert_eq!(table[3], 32.0 + 12.0 * 36.0 + 1.0);
}

#[test]
fn a_short_page_fills_with_whole_rows_and_starts_below_the_header() {
    let rt = runtime(State::fixture("tunnels", "busy").unwrap(), (1100.0, 760.0));
    let table = rect(&rt, "saved-tunnels");
    let page = rect(&rt, "tunnels-scroll");
    let rows = (table[3] - 33.0) / 36.0;
    assert!((4.0..12.0).contains(&rows), "{rows} rows");
    assert_eq!(rows.fract(), 0.0);
    assert!(table[1] + table[3] <= page[1] + page[3] + 0.5);
    let header = rect(&rt, "tunnels-columns");
    let first = rect(&rt, "grafana");
    assert!(
        first[1] >= header[1] + header[3] - 0.5,
        "{first:?} {header:?}"
    );
}

#[test]
fn the_scroll_thumb_shows_only_while_rows_exceed_the_space() {
    let short = runtime(State::fixture("tunnels", "busy").unwrap(), (1100.0, 760.0));
    let thumb = rect(&short, "tunnels-scroll-thumb");
    let rows = rect(&short, "tunnel-rows");
    assert!(
        thumb[3] > 0.0 && thumb[3] < rows[3],
        "{thumb:?} in {rows:?}"
    );
    let tall = runtime(State::fixture("tunnels", "busy").unwrap(), (1920.0, 1080.0));
    assert!(tall.find("tunnels-scroll-thumb").is_none());
}

#[test]
fn the_log_dropdown_opens_as_wide_as_its_field() {
    let mut rt = runtime(State::fixture("logs", "busy").unwrap(), (1100.0, 760.0));
    click(&mut rt, "log-tunnel-filter-trigger");
    rt.frame(1.0);
    let field = rect(&rt, "log-tunnel-filter-trigger");
    let list = rect(&rt, "log-tunnel-filter-content");
    assert!((field[2] - list[2]).abs() < 0.5, "{field:?} {list:?}");
}

#[test]
fn the_top_bar_shows_the_wordmark_only_while_the_sidebar_is_collapsed() {
    let mut rt = runtime(
        State::fixture("tunnels", "loaded").unwrap(),
        (1100.0, 760.0),
    );
    assert!(rt.find("toolbar-wordmark").is_none());
    click(&mut rt, "sidebar-toggle");
    assert!(rt.find("toolbar-wordmark").is_some());
    click(&mut rt, "sidebar-toggle");
    assert!(rt.find("toolbar-wordmark").is_none());
}

#[test]
fn the_log_filter_lists_every_tunnel_and_view_logs_preselects_one() {
    let state = State::fixture("logs", "busy").unwrap();
    let options = state.log_filters();
    assert_eq!(options[0].id, "");
    assert_eq!(options[0].label, "All tunnels");
    let ids: Vec<_> = options[1..].iter().map(|o| o.id.as_str()).collect();
    let mut expected: Vec<_> = state
        .model
        .tunnels
        .iter()
        .map(|t| t.name.as_str())
        .collect();
    expected.push("bastion-1");
    assert_eq!(ids, expected);
    assert_eq!(options.last().unwrap().detail, "Terminal");

    let mut rt = runtime(
        State::fixture("tunnels", "loaded").unwrap(),
        (1100.0, 760.0),
    );
    click(&mut rt, "view-tunnel-logs");
    assert_eq!(rt.state.logs.tunnel, "grafana");
    assert!(
        rt.state
            .filtered_logs()
            .iter()
            .all(|entry| entry.line.contains("grafana"))
    );
    click(&mut rt, "log-tunnel-filter-trigger");
    click(&mut rt, "log-tunnel-filter-option-");
    assert_eq!(rt.state.logs.tunnel, "");
}
