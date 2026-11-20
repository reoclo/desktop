use reoclo_desktop::{
    control::Client,
    fake::FakeDaemon,
    ui::{Fault, State, view},
};
use rune::{Env, Key, KeyEvent, Mods, Runtime};

fn runtime(state: State) -> Runtime<State> {
    let mut rt = Runtime::new(state, view, (1100.0, 900.0), Env::new(false, 1.0));
    rt.frame(0.0);
    rt
}

fn click(rt: &mut Runtime<State>, key: &str) {
    let (x, y) = rt.find(key).unwrap();
    rt.pointer_down(x, y);
    rt.pointer_up(x, y);
    rt.frame(0.02);
}

#[test]
fn all_tunnels_clears_the_selection_and_combines_terminal_traffic() {
    let mut rt = runtime(State::fixture("tunnels", "traffic-selected").unwrap());
    click(&mut rt, "all-tunnels");
    assert!(rt.state.model.selected.is_none());
    let now = rt.state.clock_epoch.unix_timestamp();
    let all = rt.state.model.history.series(None, now);
    let saved = rt.state.model.history.series(Some("grafana"), now);
    assert_eq!(saved[899].bytes_in, 19_000);
    assert_eq!(all[899].bytes_in, 26_500);
    assert_eq!(rt.state.model.history.retained("terminal:4242"), 900);
}

#[test]
fn retry_now_recovers_the_selected_tunnel_through_the_control_socket() {
    let dir = tempfile::tempdir().unwrap();
    let fake = FakeDaemon::start(dir.path().join("retry.sock")).unwrap();
    let client = Client::new(fake.path());
    let mut rt = runtime(State::fixture("tunnels", "retrying").unwrap());
    click(&mut rt, "retry-tunnel");
    let request = rt.state.outbox.pop().unwrap();
    let reply = client
        .request(&request.method, &request.path, request.body.as_ref())
        .map_err(Fault::from);
    rt.state.receive(&request, reply);
    assert_eq!(
        rt.state.selected().unwrap().state,
        reoclo_desktop::model::TunnelState::Ready
    );
    assert!(rt.state.selected().unwrap().retry.is_none());
}

#[test]
fn domain_validation_updates_while_typing_before_submission() {
    let mut rt = runtime(State::fixture("certificates", "loaded").unwrap());
    click(&mut rt, "trust-domain-input");
    rt.key(KeyEvent {
        key: Key::Text("app.".into()),
        mods: Mods::default(),
    });
    rt.frame(0.02);
    click(&mut rt, "trust-domain");
    assert!(rt.state.outbox.is_empty());
    click(&mut rt, "trust-domain-input");
    rt.key(KeyEvent {
        key: Key::End,
        mods: Mods::default(),
    });
    rt.key(KeyEvent {
        key: Key::Text("test".into()),
        mods: Mods::default(),
    });
    rt.frame(0.02);
    click(&mut rt, "trust-domain");
    assert_eq!(
        rt.state.outbox.last().unwrap().body.as_ref().unwrap()["domains"],
        serde_json::json!(["app.test"])
    );
}

#[test]
fn the_empty_tunnels_page_keeps_a_calm_aggregate_graph() {
    let rt = runtime(State::fixture("tunnels", "empty").unwrap());
    assert!(rt.find("traffic-graph").is_some());
    assert!(rt.find("add-tunnel").is_some());
    assert!(rt.elements().iter().all(|e| !e.moving));
}
