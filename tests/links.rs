use reoclo_desktop::{
    control::{self, Client},
    fake::FakeDaemon,
    model::TunnelState,
    system::{FakeRunner, SystemAction, SystemRunner},
    ui::{Effect, Fault, LinkStep, Request, SetupStatus, State, ViewState, read_preferences, view},
};
use rune::{Env, Runtime};
use serde_json::json;
use std::sync::mpsc;

const HTTPS_GRAFANA: &str = "reoclo://open?v=1&org=demo&server=prod-1&port=3000&kind=https";

fn open(url: &str) -> State {
    let mut state = State::fixture("tunnels", "loaded").unwrap();
    state.open_link(url);
    state
}

fn runtime(state: State) -> Runtime<State> {
    let mut rt = Runtime::new(state, view, (1100.0, 760.0), Env::new(false, 1.0));
    rt.frame(0.0);
    for _ in 0..30 {
        rt.frame(1.0 / 60.0);
    }
    rt
}

fn click(rt: &mut Runtime<State>, key: &str) {
    let (x, y) = rt.find(key).unwrap();
    rt.pointer_down(x, y);
    rt.pointer_up(x, y);
    for _ in 0..3 {
        rt.frame(1.0 / 60.0);
    }
}

/// Answers every queued request from the fake daemon, in order, and returns what was asked.
fn drive(state: &mut State, client: &Client) -> Vec<Request> {
    let mut seen = Vec::new();
    while !state.outbox.is_empty() {
        let request = state.outbox.remove(0);
        let response = client
            .request(&request.method, &request.path, request.body.as_ref())
            .map_err(Fault::from);
        state.receive(&request, response);
        seen.push(request);
    }
    seen
}

fn daemon() -> (tempfile::TempDir, FakeDaemon) {
    let dir = tempfile::tempdir().unwrap();
    let fake = FakeDaemon::start(dir.path().join("links.sock")).unwrap();
    (dir, fake)
}

fn paths(seen: &[Request]) -> Vec<String> {
    seen.iter()
        .map(|r| format!("{} {}", r.method, r.path))
        .collect()
}

#[test]
fn a_link_asks_before_it_does_anything_and_cancel_drops_it() {
    let mut rt = runtime(open(HTTPS_GRAFANA));
    assert_eq!(rt.state.modal.as_deref(), Some("open-link"));
    assert_eq!(rt.state.link_step(), Some(LinkStep::Confirm));
    let text = rt.state.link_description().unwrap();
    assert!(text.contains("HTTPS"));
    assert!(text.contains("127.0.0.1:3000"));
    assert!(text.contains("prod-1"));
    assert!(text.contains("demo"));
    assert!(rt.state.outbox.is_empty());
    click(&mut rt, "cancel-confirmation");
    assert_eq!(rt.state.modal, None);
    assert!(rt.state.link.is_none());
    assert!(rt.state.outbox.is_empty());
    assert!(rt.state.take_effects().is_empty());
}

#[test]
fn a_tcp_link_says_it_is_a_local_port() {
    let state = open("reoclo://open?v=1&org=demo&server=prod-1&port=5432&kind=tcp");
    let text = state.link_description().unwrap();
    assert!(text.contains("local port"));
    assert!(text.contains("clipboard"));
}

#[test]
fn an_invalid_link_shows_an_alert_and_does_nothing_else() {
    let mut state = State::fixture("settings", "loaded").unwrap();
    state.open_link("reoclo://open?v=1&org=demo");
    assert_eq!(
        state.error.as_deref(),
        Some("This link is missing the server.")
    );
    assert_eq!(state.modal, None);
    assert!(state.link.is_none());
    assert!(state.outbox.is_empty());
}

#[test]
fn confirming_while_signed_in_asks_for_the_organizations() {
    let mut rt = runtime(open(HTTPS_GRAFANA));
    click(&mut rt, "confirm-action");
    assert_eq!(rt.state.modal, None);
    assert_eq!(rt.state.link_step(), Some(LinkStep::Orgs));
    let request = rt.state.outbox.pop().unwrap();
    assert_eq!(
        (request.method.as_str(), request.path.as_str()),
        ("GET", "/v1/orgs")
    );
}

#[test]
fn confirming_while_signed_out_signs_in_and_continues_after_success() {
    let mut state = open(HTTPS_GRAFANA);
    state.model.status.signed_in = Some(false);
    state.confirm_link();
    assert_eq!(state.link_step(), Some(LinkStep::SignIn));
    let login = state.outbox.pop().unwrap();
    assert_eq!(
        (login.method.as_str(), login.path.as_str()),
        ("POST", "/v1/auth/login")
    );
    assert!(state.outbox.is_empty());

    state.apply_event(
        "auth",
        r#"{"status":"pending","user_code":"DEMO-7294","verification_uri":"https://auth.reoclo.com/device"}"#,
    );
    assert_eq!(state.link_step(), Some(LinkStep::SignIn));
    assert!(state.outbox.is_empty());

    state.apply_event("auth", r#"{"status":"success"}"#);
    assert_eq!(state.link_step(), Some(LinkStep::Orgs));
    assert!(state.signed_in());
    let next = state.outbox.pop().unwrap();
    assert_eq!(
        (next.method.as_str(), next.path.as_str()),
        ("GET", "/v1/orgs")
    );
}

#[test]
fn a_failed_sign_in_drops_the_link() {
    let mut state = open(HTTPS_GRAFANA);
    state.model.status.signed_in = Some(false);
    state.confirm_link();
    state.apply_event(
        "auth",
        r#"{"status":"failed","message":"Sign-in was denied."}"#,
    );
    assert!(state.link.is_none());
    assert_eq!(state.error.as_deref(), Some("Sign-in was denied."));
}

#[test]
fn confirming_with_the_service_not_running_opens_get_started() {
    let mut state = State::fixture("tunnels", "offline").unwrap();
    state.open_link(HTTPS_GRAFANA);
    state.confirm_link();
    assert!(state.link.is_none());
    assert_eq!(state.modal.as_deref(), Some("get-started"));
    assert!(state.error.as_deref().unwrap().contains("not running"));
}

#[test]
fn an_organization_the_account_cannot_see_is_an_error() {
    let (_dir, fake) = daemon();
    let client = Client::new(fake.path());
    let mut state = open("reoclo://open?v=1&org=nope&server=prod-1&port=3000&kind=https");
    state.confirm_link();
    let seen = drive(&mut state, &client);
    assert_eq!(paths(&seen), ["GET /v1/orgs"]);
    assert_eq!(
        state.error.as_deref(),
        Some("Your Reoclo account does not have access to nope.")
    );
    assert!(state.link.is_none());
}

#[test]
fn a_server_that_is_not_in_the_organization_is_an_error() {
    let (_dir, fake) = daemon();
    let client = Client::new(fake.path());
    let mut state = open("reoclo://open?v=1&org=demo&server=ghost&port=3000&kind=https");
    state.confirm_link();
    let seen = drive(&mut state, &client);
    assert_eq!(paths(&seen), ["GET /v1/orgs", "GET /v1/orgs/demo/servers"]);
    assert_eq!(
        state.error.as_deref(),
        Some("Reoclo could not find a server called ghost in demo.")
    );
    assert!(state.link.is_none());
}

#[test]
fn a_server_matches_by_slug_or_by_id() {
    for server in ["prod-1", "server-1"] {
        let (_dir, fake) = daemon();
        let client = Client::new(fake.path());
        let mut state = open(&format!(
            "reoclo://open?v=1&org=demo&server={server}&port=3000&kind=https"
        ));
        state.confirm_link();
        drive(&mut state, &client);
        assert_eq!(state.error, None, "{server}");
        assert_eq!(
            state.take_effects(),
            [Effect::OpenUrl(
                "https://grafana.prod-1.reoclo.localhost:8443".into()
            )]
        );
    }
}

#[test]
fn a_saved_tunnel_with_the_same_server_kind_and_target_is_reused() {
    let (_dir, fake) = daemon();
    let client = Client::new(fake.path());
    let mut state = open(HTTPS_GRAFANA);
    state.confirm_link();
    let seen = drive(&mut state, &client);
    assert_eq!(paths(&seen), ["GET /v1/orgs", "GET /v1/orgs/demo/servers"]);
    assert_eq!(state.model.tunnels.len(), 3);
    assert!(state.link.is_none());
    assert_eq!(state.model.selected.as_deref(), Some("grafana"));
    assert_eq!(
        state.take_effects(),
        [Effect::OpenUrl(
            "https://grafana.prod-1.reoclo.localhost:8443".into()
        )]
    );
}

#[test]
fn a_saved_tunnel_that_is_down_is_brought_up_and_its_endpoint_copied() {
    let (_dir, fake) = daemon();
    let client = Client::new(fake.path());
    let mut state = open("reoclo://open?v=1&org=demo&server=staging-1&port=6379&kind=tcp");
    state.confirm_link();
    let seen = drive(&mut state, &client);
    assert_eq!(
        paths(&seen),
        [
            "GET /v1/orgs",
            "GET /v1/orgs/demo/servers",
            "POST /v1/tunnels/redis/up"
        ]
    );
    assert_eq!(state.model.tunnels.len(), 3);
    assert_eq!(
        state.take_effects(),
        [Effect::Copy("127.0.0.1:16379".into())]
    );
    assert!(state.notice.as_deref().unwrap().contains("127.0.0.1:16379"));
}

#[test]
fn a_new_https_tunnel_is_created_with_the_form_body() {
    let (_dir, fake) = daemon();
    let client = Client::new(fake.path());
    let mut state = open(
        "reoclo://open?v=1&org=demo&server=prod-1&port=8080&kind=https&name=api-web&host=10.0.0.4",
    );
    state.orgs[0].id = "tenant-1".into();
    state.confirm_link();
    let orgs = state.outbox.remove(0);
    state.receive(
        &orgs,
        Ok(json!({"orgs": [{"id": "tenant-1", "slug": "demo", "name": "Demo"}]})),
    );
    let seen = drive(&mut state, &client);
    let post = seen.last().unwrap();
    assert_eq!(
        (post.method.as_str(), post.path.as_str()),
        ("POST", "/v1/tunnels")
    );
    assert_eq!(
        post.body.clone().unwrap(),
        json!({
            "name": "api-web",
            "org": "demo",
            "server": "prod-1",
            "server_id": "server-1",
            "kind": "https",
            "via": "reoclo",
            "tenant_id": "tenant-1",
            "target": "10.0.0.4:8080",
            "hostname": "api-web.prod-1.reoclo.localhost",
        })
    );
    assert_eq!(
        state.take_effects(),
        [Effect::OpenUrl(
            "https://api-web.prod-1.reoclo.localhost:8443".into()
        )]
    );
}

#[test]
fn a_new_tcp_tunnel_gets_a_local_port_and_no_hostname() {
    let (_dir, fake) = daemon();
    let client = Client::new(fake.path());
    let mut state = open("reoclo://open?v=1&org=demo&server=prod-1&port=9000&kind=tcp");
    state.confirm_link();
    let seen = drive(&mut state, &client);
    let body = seen.last().unwrap().body.clone().unwrap();
    assert_eq!(body["kind"], "tcp");
    assert_eq!(body["target"], "127.0.0.1:9000");
    assert_eq!(body["local_port"], 9000);
    assert_eq!(body["name"], "prod-1-9000");
    assert_eq!(body["tenant_id"], "demo");
    assert!(body.get("hostname").is_none());
    assert_eq!(
        state.take_effects(),
        [Effect::Copy("127.0.0.1:9000".into())]
    );
}

#[test]
fn a_taken_name_gets_a_numeric_suffix() {
    let (_dir, fake) = daemon();
    let client = Client::new(fake.path());
    let mut state =
        open("reoclo://open?v=1&org=demo&server=prod-1&port=8080&kind=https&name=grafana");
    state.confirm_link();
    let seen = drive(&mut state, &client);
    assert_eq!(
        seen.last().unwrap().body.clone().unwrap()["name"],
        "grafana-2"
    );
    assert!(state.model.tunnels.iter().any(|t| t.name == "grafana-2"));
    assert!(state.error.is_none());
}

#[test]
fn a_daemon_error_shows_as_an_alert_and_drops_the_link() {
    let mut state = open("reoclo://open?v=1&org=demo&server=prod-1&port=8080&kind=https");
    state.confirm_link();
    let orgs = state.outbox.remove(0);
    state.receive(
        &orgs,
        Ok(json!({"orgs": [{"id": "demo", "slug": "demo", "name": "Demo"}]})),
    );
    let servers = state.outbox.remove(0);
    state.receive(
        &servers,
        Ok(json!({"servers": [{"id": "server-1", "slug": "prod-1", "name": "Production"}]})),
    );
    let post = state.outbox.remove(0);
    assert_eq!(post.path, "/v1/tunnels");
    let fault = Fault::from(control::Error::Api {
        status: 409,
        code: "conflict".into(),
        message: "A tunnel already uses that name".into(),
        state: None,
    });
    state.receive(&post, Err(fault));
    assert!(
        state
            .error
            .as_deref()
            .unwrap()
            .contains("A tunnel already uses that name")
    );
    assert!(state.link.is_none());
}

#[test]
fn a_tunnel_still_connecting_waits_for_the_daemon_event() {
    let (_dir, fake) = daemon();
    let client = Client::new(fake.path());
    let mut state = open(HTTPS_GRAFANA);
    state.model.tunnels[0].state = TunnelState::Connecting;
    state.confirm_link();
    drive(&mut state, &client);
    assert_eq!(state.link_step(), Some(LinkStep::Reaching));
    assert!(state.take_effects().is_empty());

    let event = |status: &str| {
        json!({"type": "tunnel", "name": "grafana", "tunnel": {
            "name": "grafana", "org": "demo", "server": "prod-1", "server_id": "server-1",
            "kind": "https", "state": status, "target": "127.0.0.1:3000",
            "url": "https://grafana.prod-1.reoclo.localhost:8443"
        }})
        .to_string()
    };
    state.apply_event("tunnel", &event("reconnecting"));
    assert_eq!(state.link_step(), Some(LinkStep::Reaching));
    state.apply_event("tunnel", &event("active"));
    assert!(state.link.is_none());
    assert_eq!(
        state.take_effects(),
        [Effect::OpenUrl(
            "https://grafana.prod-1.reoclo.localhost:8443".into()
        )]
    );
}

#[test]
fn a_tunnel_that_fails_to_come_up_is_an_error() {
    let (_dir, fake) = daemon();
    let client = Client::new(fake.path());
    let mut state = open(HTTPS_GRAFANA);
    state.model.tunnels[0].state = TunnelState::Connecting;
    state.confirm_link();
    drive(&mut state, &client);
    state.apply_event(
        "tunnel",
        &json!({"type": "tunnel", "name": "grafana", "tunnel": {
            "name": "grafana", "server_id": "server-1", "kind": "https", "state": "server_offline",
            "message": "The server is offline", "target": "127.0.0.1:3000"
        }})
        .to_string(),
    );
    assert!(state.link.is_none());
    assert!(
        state
            .error
            .as_deref()
            .unwrap()
            .contains("The server is offline")
    );
    assert!(state.take_effects().is_empty());
}

#[test]
fn a_link_that_never_hears_back_is_dropped() {
    let mut state = open(HTTPS_GRAFANA);
    state.model.tunnels[0].state = TunnelState::Connecting;
    let (_dir, fake) = daemon();
    let client = Client::new(fake.path());
    state.confirm_link();
    drive(&mut state, &client);
    let mut tx = rune::Tx::default();
    assert!(!state.poll(100.0, &mut tx));
    assert!(!state.poll(150.0, &mut tx));
    assert!(state.link.is_some());
    assert!(state.poll(161.0, &mut tx));
    assert!(state.link.is_none());
    assert!(state.error.is_some());
}

#[test]
fn https_without_a_trusted_ca_trusts_it_first_and_then_continues() {
    let (_dir, fake) = daemon();
    let client = Client::new(fake.path());
    let mut state = open("reoclo://open?v=1&org=demo&server=prod-1&port=8080&kind=https");
    if let Some(ca) = &mut state.model.status.ca {
        ca.trusted = false;
    }
    state.confirm_link();
    let seen = drive(&mut state, &client);
    assert_eq!(
        paths(&seen),
        [
            "GET /v1/orgs",
            "GET /v1/orgs/demo/servers",
            "POST /v1/ca/trust",
            "POST /v1/tunnels"
        ]
    );
    assert!(state.ca_ready());
    assert_eq!(state.take_effects().len(), 1);
}

#[test]
fn https_without_the_hosts_helper_installs_it_before_continuing() {
    let (_dir, fake) = daemon();
    let client = Client::new(fake.path());
    let mut state = open("reoclo://open?v=1&org=demo&server=prod-1&port=8080&kind=https");
    if let Some(ca) = &mut state.model.status.ca {
        ca.trusted = false;
    }
    if let Some(hosts) = &mut state.model.status.hosts {
        hosts.installed = false;
    }
    let (runner, actions) = mpsc::channel();
    state.system_runner = Some(runner);
    state.confirm_link();
    let seen = drive(&mut state, &client);
    assert_eq!(paths(&seen).last().unwrap(), "POST /v1/ca/trust");
    assert_eq!(state.link_step(), Some(LinkStep::Secure));
    assert!(matches!(
        actions.try_recv(),
        Ok((SystemAction::SetupHosts(_), Some(_)))
    ));

    state.awaiting_helper = true;
    state.receive(
        &Request {
            method: "POST".into(),
            path: "/v1/hosts/sync".into(),
            body: None,
        },
        Ok(json!({"entries": [], "helper": {"installed": true, "entries": 0, "relay": {"enabled": false, "listening": false}}})),
    );
    let seen = drive(&mut state, &client);
    assert_eq!(paths(&seen), ["POST /v1/tunnels"]);
    assert_eq!(state.take_effects().len(), 1);
}

#[test]
fn a_helper_install_that_is_refused_drops_the_link() {
    let mut state = open("reoclo://open?v=1&org=demo&server=prod-1&port=8080&kind=https");
    if let Some(ca) = &mut state.model.status.ca {
        ca.trusted = false;
    }
    if let Some(hosts) = &mut state.model.status.hosts {
        hosts.installed = false;
    }
    let (runner, _actions) = mpsc::channel();
    state.system_runner = Some(runner);
    state.confirm_link();
    let (_dir, fake) = daemon();
    drive(&mut state, &Client::new(fake.path()));
    assert_eq!(state.link_step(), Some(LinkStep::Secure));
    state.system_failed(&SystemAction::SetupHosts(Default::default()));
    assert!(state.link.is_none());
    assert!(!state.securing);
}

#[test]
fn get_started_reports_where_each_step_stands() {
    let status = |page: &str| State::fixture(page, "loaded").unwrap().setup_status();
    assert_eq!(
        status("get-started"),
        [
            SetupStatus::Needed,
            SetupStatus::Waiting,
            SetupStatus::Waiting
        ]
    );
    assert_eq!(
        status("get-started-progress"),
        [
            SetupStatus::Done,
            SetupStatus::Needed,
            SetupStatus::Optional
        ]
    );
    assert_eq!(
        status("get-started-done"),
        [SetupStatus::Done, SetupStatus::Done, SetupStatus::Done]
    );
    let mut state = State::fixture("get-started-done", "loaded").unwrap();
    state.service_installed = false;
    assert_eq!(state.setup_status()[0], SetupStatus::Needed);
    let mut loading = State::fixture("get-started-done", "loaded").unwrap();
    loading.phase = ViewState::Loading;
    assert_eq!(loading.setup_status(), [SetupStatus::Checking; 3]);
}

#[test]
fn the_service_step_runs_the_install_and_the_fake_runner_makes_no_changes() {
    let mut state = State::fixture("get-started", "loaded").unwrap();
    let (runner, actions) = mpsc::channel();
    state.system_runner = Some(runner);
    let mut rt = runtime(state);
    click(&mut rt, "setup-install-service");
    let (action, cli) = actions.try_recv().unwrap();
    assert_eq!(action, SystemAction::InstallDaemon);
    assert_eq!(
        cli.as_deref(),
        Some(std::path::Path::new("/opt/homebrew/bin/reoclo"))
    );
    assert!(
        FakeRunner
            .run(&action, cli.as_deref())
            .unwrap()
            .contains("no system changes")
    );
}

#[test]
fn the_offline_start_button_installs_the_service() {
    let mut state = State::fixture("tunnels", "offline").unwrap();
    let (runner, actions) = mpsc::channel();
    state.system_runner = Some(runner);
    let mut rt = runtime(state);
    click(&mut rt, "state-action");
    for _ in 0..30 {
        rt.frame(1.0 / 60.0);
    }
    click(&mut rt, "confirm-action");
    assert_eq!(actions.try_recv().unwrap().0, SystemAction::InstallDaemon);
}

#[test]
fn the_sign_in_step_asks_for_a_device_code_and_shows_it() {
    let rt = runtime(State::fixture("get-started-progress", "loaded").unwrap());
    assert!(rt.find("setup-open-sign-in").is_some());
    assert!(rt.find("setup-sign-in").is_none());

    let mut state = State::fixture("get-started-progress", "loaded").unwrap();
    state.model.auth = Default::default();
    let mut rt = runtime(state);
    click(&mut rt, "setup-sign-in");
    let request = rt.state.outbox.pop().unwrap();
    assert_eq!(
        (request.method.as_str(), request.path.as_str()),
        ("POST", "/v1/auth/login")
    );
}

#[test]
fn the_secure_step_trusts_the_certificate_then_finishes() {
    let mut state = State::fixture("get-started-progress", "loaded").unwrap();
    state.model.auth = Default::default();
    let mut rt = runtime(state);
    click(&mut rt, "setup-secure");
    let request = rt.state.outbox.pop().unwrap();
    assert_eq!(request.path, "/v1/ca/trust");
    rt.state.receive(&request, Ok(json!({"ok": true})));
    assert!(rt.state.secure_ready());
    assert_eq!(
        rt.state.notice.as_deref(),
        Some("Secure web addresses are ready.")
    );
    assert!(!rt.state.securing);
}

#[test]
fn done_closes_get_started_and_remembers_it() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("preferences.json");
    let mut state = State::fixture("get-started-done", "loaded").unwrap();
    state.preferences_path = Some(path.clone());
    assert!(!read_preferences(&path).onboarded);
    let mut rt = runtime(state);
    click(&mut rt, "setup-done");
    assert_eq!(rt.state.modal, None);
    assert!(rt.state.onboarded);
    assert!(read_preferences(&path).onboarded);
}

#[test]
fn settings_can_run_setup_again() {
    let mut rt = runtime(State::fixture("settings", "loaded").unwrap());
    click(&mut rt, "run-setup");
    assert_eq!(rt.state.modal.as_deref(), Some("get-started"));
    let request = rt.state.outbox.pop().unwrap();
    assert_eq!(request.path, "/v1/status");
}
