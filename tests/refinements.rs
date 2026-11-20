use reoclo_desktop::ui::{Fault, Request, State, ViewState, view};
use rune::{Env, Runtime};
use std::sync::mpsc;

fn runtime(state: State) -> Runtime<State> {
    let mut rt = Runtime::new(state, view, (1100.0, 760.0), Env::new(false, 1.0));
    rt.frame(0.0);
    rt
}

fn click(rt: &mut Runtime<State>, key: &str) {
    if key.starts_with("toggle-row-") {
        rt.update(|_, tx| tx.scroll_into_view("saved-tunnels"));
        for _ in 0..60 {
            rt.frame(1.0 / 60.0);
        }
    }
    rt.update(|_, tx| tx.scroll_into_view(key));
    for _ in 0..60 {
        rt.frame(1.0 / 60.0);
    }
    let (x, y) = rt.find(key).unwrap();
    rt.pointer_down(x, y);
    rt.pointer_up(x, y);
    rt.frame(0.02);
}

#[test]
fn failed_tunnel_puts_recovery_before_facts_and_never_offers_open() {
    let rt = runtime(State::fixture("tunnels", "failed").unwrap());
    assert!(rt.find("open-endpoint").is_none());
    let retry = rt.find("retry-tunnel").unwrap().1;
    assert!(retry < rt.find("tunnel-facts").unwrap().1);
}

#[test]
fn independent_switches_remain_usable_while_one_request_is_pending() {
    let mut state = State::fixture("tunnels", "loaded").unwrap();
    let (sender, receiver) = mpsc::channel();
    state.sender = Some(sender);
    let mut rt = runtime(state);
    click(&mut rt, "toggle-row-grafana");
    click(&mut rt, "toggle-row-postgres");
    let paths: Vec<_> = receiver.try_iter().map(|r| r.path).collect();
    assert_eq!(
        paths,
        ["/v1/tunnels/grafana/down", "/v1/tunnels/postgres/down"]
    );
}

#[test]
fn approval_places_the_primary_action_rightmost() {
    let rt = runtime(State::fixture("tunnels", "loaded").unwrap());
    assert!(rt.find("deny-approval-1").unwrap().0 < rt.find("approve-approval-1").unwrap().0);
}

#[test]
fn losing_the_daemon_keeps_cached_rows_without_live_switches() {
    let mut state = State::fixture("tunnels", "loaded").unwrap();
    state.receive(
        &Request {
            method: "GET".into(),
            path: "/v1/tunnels".into(),
            body: None,
        },
        Err(Fault::Offline("Connection refused".into())),
    );
    assert_eq!(state.phase, ViewState::Offline);
    let rt = runtime(state);
    assert!(rt.find("saved-tunnels").is_some());
    assert!(rt.find("toggle-row-grafana").is_none());
}

#[test]
fn a_shared_failure_retries_every_member_through_the_daemon() {
    use reoclo_desktop::{control::Client, fake::FakeDaemon, model::TunnelState};
    let dir = tempfile::tempdir().unwrap();
    let fake = FakeDaemon::start(dir.path().join("retry.sock")).unwrap();
    let client = Client::new(fake.path());
    let mut state = State::fixture("tunnels", "loaded").unwrap();
    for tunnel in state.model.tunnels.iter_mut().take(2) {
        tunnel.state = TunnelState::Failed;
        tunnel.detail = Some("A shared listener failed.".into());
    }
    state.model.tunnels[2].state = TunnelState::Failed;
    state.model.tunnels[2].detail = Some("A different listener failed.".into());
    let mut rt = runtime(state);
    assert_eq!(rt.state.shared_failures().len(), 1);
    click(&mut rt, "retry-group-0");
    for request in std::mem::take(&mut rt.state.outbox) {
        let result = client
            .request(&request.method, &request.path, request.body.as_ref())
            .map_err(Fault::from);
        rt.state.receive(&request, result);
    }
    let states: Vec<_> = rt.state.model.tunnels.iter().map(|t| t.state).collect();
    assert_eq!(
        states,
        [TunnelState::Ready, TunnelState::Ready, TunnelState::Failed]
    );
}

#[test]
fn changing_the_https_port_opens_settings_with_keyboard_focus() {
    let mut rt = runtime(State::fixture("tunnels", "shared-failure").unwrap());
    click(&mut rt, "change-https-port-0");
    for _ in 0..60 {
        rt.frame(1.0 / 60.0);
    }
    assert_eq!(rt.state.page, reoclo_desktop::ui::Page::Settings);
    assert!(
        rt.elements()
            .iter()
            .any(|e| e.label.as_deref() == Some("HTTPS port") && e.focused)
    );
}

#[test]
fn only_listening_tunnels_offer_open() {
    for case in [
        "active",
        "ready",
        "down",
        "connecting",
        "reconnecting",
        "needs_sign_in",
        "server_offline",
        "quota_reached",
        "expired",
        "closed_by_operator",
        "forbidden",
        "failed",
    ] {
        let rt = runtime(State::fixture("tunnels", case).unwrap());
        assert_eq!(
            rt.find("open-endpoint").is_some(),
            matches!(case, "active" | "ready"),
            "{case}"
        );
        assert!(rt.find("copy-endpoint").is_some(), "{case}");
    }
}

#[test]
fn a_state_event_preserves_the_users_sorted_row_order() {
    let mut state = State::fixture("tunnels", "loaded").unwrap();
    state.model.tunnels.reverse();
    let names: Vec<_> = state.model.tunnels.iter().map(|t| t.name.clone()).collect();
    let mut changed = state.model.tunnels[0].clone();
    changed.state = reoclo_desktop::model::TunnelState::Failed;
    state.apply_event(
        "tunnel",
        &serde_json::json!({"name":changed.name,"tunnel":changed}).to_string(),
    );
    assert_eq!(
        state
            .model
            .tunnels
            .iter()
            .map(|t| t.name.clone())
            .collect::<Vec<_>>(),
        names
    );
}

#[test]
fn compact_long_names_keep_the_switch_inside_the_window() {
    let state = State::fixture("tunnels", "long-names").unwrap();
    let mut rt = Runtime::new(state, view, (760.0, 620.0), Env::new(false, 1.0));
    for _ in 0..60 {
        rt.frame(1.0 / 60.0);
    }
    assert!(!rt.state.sidebar.is_open());
    for e in rt.elements().iter().filter(|e| e.role == "switch") {
        assert!(
            e.rect[0] >= 0.0 && e.rect[0] + e.rect[2] <= 760.0,
            "{:?}",
            e.rect
        );
    }
}

#[test]
fn selecting_a_tunnel_reveals_its_header_without_scrolling_the_page_title() {
    let mut rt = runtime(State::fixture("tunnels", "shared-failure").unwrap());
    let header = rt.find("tunnels-header").unwrap();
    rt.update(|s, tx| s.reveal_tunnel("postgres", tx));
    for _ in 0..60 {
        rt.frame(1.0 / 60.0);
    }
    assert_eq!(rt.find("tunnels-header").unwrap(), header);
    let detail = rt
        .elements()
        .into_iter()
        .find(|e| e.key.as_deref() == Some("tunnel-header"))
        .unwrap();
    assert!(detail.visible[3] > 0.0);
    assert!(detail.rect[1] + detail.rect[3] <= 760.0);
}

#[test]
fn tunnel_events_exclude_other_tunnels_and_similarly_named_tunnels() {
    let mut rt = runtime(State::fixture("tunnels", "events").unwrap());
    click(&mut rt, "view-tunnel-logs");
    let lines: Vec<_> = rt
        .elements()
        .into_iter()
        .filter_map(|e| e.text)
        .filter(|text| text.contains(" UTC "))
        .collect();
    assert_eq!(lines.len(), 2);
    assert!(lines.iter().all(|line| line.contains("grafana:")));
    assert!(rt.find("copy-events").is_some());
}

#[test]
fn approval_expiry_advances_on_the_virtual_clock() {
    let mut state = State::fixture("tunnels", "loaded").unwrap();
    let mut tx = rune::Tx::default();
    state.poll(0.0, &mut tx);
    state.poll(61.0, &mut tx);
    let rt = runtime(state);
    assert!(
        rt.elements()
            .iter()
            .any(|e| e.text.as_deref() == Some("Expires in 3 min"))
    );
}

#[test]
fn an_expired_approval_leaves_the_list_and_the_count() {
    let mut state = State::fixture("tunnels", "loaded").unwrap();
    let mut tx = rune::Tx::default();
    state.poll(0.0, &mut tx);
    state.poll(241.0, &mut tx);
    assert_eq!(state.pending_approvals().count(), 0);
    let rt = runtime(state);
    assert!(rt.find("approve-approval-1").is_none());
    assert!(rt.find("deny-approval-1").is_none());
}
