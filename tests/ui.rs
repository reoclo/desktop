use reoclo_desktop::ui::{State, view};
use reoclo_desktop::{control::Client, fake::FakeDaemon};
use rune::{Env, Runtime};

#[test]
fn selecting_tunnel_and_toggling_queues_the_correct_daemon_action() {
    let mut rt = Runtime::new(
        State::fixture("tunnels", "loaded").unwrap(),
        view,
        (1100.0, 760.0),
        Env::new(false, 1.0),
    );
    rt.frame(0.0);
    let (x, y) = rt.find("toggle-selected").unwrap();
    rt.pointer_down(x, y);
    rt.pointer_up(x, y);
    rt.frame(0.02);
    let action = rt.state.outbox.pop().unwrap();
    assert_eq!(action.path, "/v1/tunnels/grafana/down");
    assert_eq!(action.method, "POST");
}

#[test]
fn save_tunnel_click_reaches_fake_daemon_and_updates_the_visible_model() {
    let dir = tempfile::tempdir().unwrap();
    let fake = FakeDaemon::start(dir.path().join("ui.sock")).unwrap();
    let client = Client::new(fake.path());
    let mut rt = Runtime::new(
        State::fixture("add", "loaded").unwrap(),
        view,
        (1100.0, 760.0),
        Env::new(false, 1.0),
    );
    rt.frame(0.0);
    let (x, y) = rt.find("save-tunnel").unwrap();
    rt.pointer_down(x, y);
    rt.pointer_up(x, y);
    rt.frame(0.02);
    let request = rt.state.outbox.pop().unwrap();
    let response = client
        .request(&request.method, &request.path, request.body.as_ref())
        .map_err(reoclo_desktop::ui::Fault::from);
    rt.state.receive(&request, response);
    rt.frame(0.02);
    assert_eq!(rt.state.modal, None);
    assert_eq!(rt.state.selected().unwrap().name, "api");
    assert_eq!(
        rt.state.selected().unwrap().url.as_deref(),
        Some("https://api.prod-1.reoclo.localhost:8443")
    );
}

#[test]
fn scrolling_the_form_reveals_hostname_without_moving_footer() {
    let mut rt = Runtime::new(
        State::fixture("add", "loaded").unwrap(),
        view,
        (1100.0, 760.0),
        Env::new(false, 1.0),
    );
    rt.frame(0.0);
    let footer_before = rt.find("save-tunnel").unwrap();
    let hostname_before = rt.find("hostname").unwrap().1;
    let (x, y) = rt.find("form-scroll").unwrap();
    rt.wheel(x, y, -700.0);
    for _ in 0..60 {
        rt.frame(1.0 / 60.0);
    }
    assert!(rt.find("hostname").unwrap().1 < hostname_before);
    assert_eq!(rt.find("save-tunnel").unwrap(), footer_before);
}

#[test]
fn canceling_discard_returns_to_edit_without_turning_it_into_create() {
    let mut state = State::fixture("edit", "loaded").unwrap();
    state.target = rune::TextEdit::new("127.0.0.1:8080");
    state.confirm_discard();
    state.cancel_confirmation();
    assert_eq!(state.modal.as_deref(), Some("edit"));
    assert_eq!(state.target.text, "127.0.0.1:8080");
}

#[test]
fn approval_click_updates_the_pending_list_from_the_real_response_shape() {
    let dir = tempfile::tempdir().unwrap();
    let fake = FakeDaemon::start(dir.path().join("approval.sock")).unwrap();
    let client = Client::new(fake.path());
    let mut rt = Runtime::new(
        State::fixture("panel", "loaded").unwrap(),
        view,
        (340.0, 620.0),
        Env::new(false, 1.0),
    );
    rt.frame(0.0);
    let (x, y) = rt.find("approve-approval-1").unwrap();
    rt.pointer_down(x, y);
    rt.pointer_up(x, y);
    rt.frame(0.02);
    let request = rt.state.outbox.pop().unwrap();
    let response = client
        .request(&request.method, &request.path, request.body.as_ref())
        .map_err(reoclo_desktop::ui::Fault::from);
    rt.state.receive(&request, response);
    assert!(rt.state.model.approvals.is_empty());
    assert_eq!(rt.state.notice.as_deref(), Some("Request approved."));
}

#[test]
fn unauthorized_directory_response_shows_sign_in_instead_of_offline() {
    use reoclo_desktop::ui::{Fault, Request, ViewState};
    let mut state = State::default();
    let request = Request {
        method: "GET".into(),
        path: "/v1/orgs".into(),
        body: None,
    };
    let fault = Fault::from(reoclo_desktop::control::Error::Api {
        status: 401,
        code: "unauthorized".into(),
        message: "Sign in required".into(),
        state: Some("needs_sign_in".into()),
    });
    state.receive(&request, Err(fault));
    assert_eq!(state.phase, ViewState::SignedOut);
}

#[test]
fn port_picker_selects_the_service_and_preserves_its_target_port() {
    let mut rt = Runtime::new(
        State::fixture("add", "loaded").unwrap(),
        view,
        (1100.0, 760.0),
        Env::new(false, 1.0),
    );
    rt.frame(0.0);
    let (x, y) = rt.find("port-trigger").unwrap();
    rt.pointer_down(x, y);
    rt.pointer_up(x, y);
    for _ in 0..30 {
        rt.frame(1.0 / 60.0);
    }
    let (x, y) = rt.find("port-option-1").unwrap();
    rt.pointer_down(x, y);
    rt.pointer_up(x, y);
    rt.frame(0.02);
    assert_eq!(rt.state.target.text, "127.0.0.1:5432");
    assert!(!rt.state.port_picker.open);
}
