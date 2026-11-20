use reoclo_desktop::ui::{State, view};
use rune::{Env, Runtime, TextEdit};

fn click(rt: &mut Runtime<State>, key: &str) {
    let (x, y) = rt.find(key).unwrap();
    rt.pointer_down(x, y);
    rt.pointer_up(x, y);
    rt.frame(0.02);
}

#[test]
fn pause_freezes_the_filtered_view_while_live_events_continue() {
    let mut state = State::fixture("logs", "loaded").unwrap();
    state.logs.tunnel = "grafana".into();
    let mut rt = Runtime::new(state, view, (1100.0, 760.0), Env::new(false, 1.0));
    rt.frame(0.0);
    click(&mut rt, "pause-logs");
    rt.state
        .apply_event("log", r#"{"line":"grafana: new request"}"#);
    assert_eq!(rt.state.filtered_logs().len(), 2);
    click(&mut rt, "pause-logs");
    assert_eq!(rt.state.filtered_logs().len(), 3);
    rt.state.logs.query = TextEdit::new("REFUSED");
    assert_eq!(
        rt.state.filtered_logs()[0].line,
        "grafana: upstream connection refused"
    );
}

#[test]
fn long_logs_mount_only_rows_near_the_viewport() {
    let mut rt = Runtime::new(
        State::fixture("logs", "long").unwrap(),
        view,
        (1100.0, 760.0),
        Env::new(false, 1.0),
    );
    for _ in 0..60 {
        rt.frame(1.0 / 60.0);
    }
    let rows = rt
        .elements()
        .iter()
        .filter(|e| {
            e.key
                .as_deref()
                .is_some_and(|key| key.starts_with("log-") && key != "log-rows")
        })
        .count();
    assert!(rows < 200, "mounted {rows} rows");
    assert!(rt.elements().iter().any(|e| {
        e.text
            .as_ref()
            .is_some_and(|line| line.contains("request 9999 completed"))
    }));
}
