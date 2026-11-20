use reoclo_desktop::ui::{Fault, PortsScan, Request, State, ViewState, view};
use rune::{Env, Runtime};
use serde_json::{Value, json};

const PORTS_PATH: &str = "/v1/orgs/demo/servers/server-1/ports";

fn form() -> Runtime<State> {
    let mut state = State::fixture("add", "loaded").unwrap();
    state.ports.clear();
    let mut rt = Runtime::new(state, view, (1100.0, 760.0), Env::new(false, 1.0));
    rt.frame(0.0);
    rt
}

fn scan(rt: &mut Runtime<State>) -> Request {
    rt.update(|s, _| s.scan_ports());
    rt.frame(0.02);
    let request = rt.state.outbox.pop().unwrap();
    assert_eq!(request.method, "GET");
    assert_eq!(request.path, PORTS_PATH);
    request
}

fn reply(rt: &mut Runtime<State>, request: &Request, response: Result<Value, Fault>) {
    let request = request.clone();
    rt.update(move |s, _| s.receive(&request, response));
    rt.frame(0.02);
}

fn click(rt: &mut Runtime<State>, key: &str) {
    let (x, y) = rt.find(key).unwrap_or_else(|| panic!("no {key} on screen"));
    rt.pointer_down(x, y);
    rt.pointer_up(x, y);
    rt.frame(0.02);
}

/// What `GET /v1/orgs/demo/servers/server-1/ports` answers for the server whose scan lists
/// loopback, zone-suffixed and bracketed addresses.
fn generator_queue_ports() -> Value {
    json!({"ports": [
        {"port": 22, "protocol": "tcp", "bind_address": "0.0.0.0", "process": ""},
        {"port": 53, "protocol": "udp", "bind_address": "127.0.0.53%lo", "process": "systemd-resolve"},
        {"port": 631, "protocol": "tcp", "bind_address": "[::1]", "process": "cupsd"},
        {"port": 14400, "protocol": "tcp", "bind_address": "127.0.0.1", "process": "rootlesskit"},
    ]})
}

#[test]
fn a_scanned_server_lists_its_ports_and_the_picker_fills_the_target() {
    let mut rt = form();
    let request = scan(&mut rt);
    assert_eq!(rt.state.ports_scan, PortsScan::Scanning);
    assert!(rt.find("ports-status").is_some());

    reply(&mut rt, &request, Ok(generator_queue_ports()));
    assert_eq!(rt.state.ports_scan, PortsScan::Loaded);
    assert_eq!(rt.state.ports.len(), 4);
    assert!(rt.find("ports-status").is_none());

    click(&mut rt, "port-value-trigger");
    assert!(rt.state.port_picker.is_open());
    click(&mut rt, "port-option-3");
    assert_eq!(rt.state.target.text, "127.0.0.1:14400");
}

#[test]
fn a_server_with_nothing_listening_says_so_and_keeps_the_target_field() {
    let mut rt = form();
    let request = scan(&mut rt);
    reply(&mut rt, &request, Ok(json!({"ports": []})));
    assert_eq!(rt.state.ports_scan, PortsScan::Loaded);
    assert!(rt.find("ports-status").is_some());
    assert!(rt.find("ports-retry").is_none());
    assert!(rt.find("target").is_some());
}

#[test]
fn a_failed_scan_says_why_and_offers_another_try() {
    let mut rt = form();
    let request = scan(&mut rt);
    reply(
        &mut rt,
        &request,
        Err(Fault::Error("Runner is offline".into())),
    );
    assert_eq!(
        rt.state.ports_scan,
        PortsScan::Failed("Runner is offline".into())
    );
    assert_eq!(rt.state.phase, ViewState::Loaded);
    assert!(rt.find("ports-status").is_some());
    assert!(rt.find("target").is_some());

    click(&mut rt, "ports-retry");
    let retry = rt.state.outbox.pop().unwrap();
    assert_eq!(retry.path, PORTS_PATH);
    assert_eq!(rt.state.ports_scan, PortsScan::Scanning);
    assert!(rt.find("ports-retry").is_none());
}

#[test]
fn a_port_list_the_app_cannot_read_fails_the_scan_and_not_the_form() {
    let mut rt = form();
    let request = scan(&mut rt);
    reply(&mut rt, &request, Ok(json!({"ports": "none"})));
    assert!(matches!(rt.state.ports_scan, PortsScan::Failed(_)));
    assert_eq!(rt.state.phase, ViewState::Loaded);
    assert!(rt.find("ports-retry").is_some());
}
