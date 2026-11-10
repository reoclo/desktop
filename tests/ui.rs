use reoclo_desktop::ui::{Fault, Page, Request, State, view};
use reoclo_desktop::{cli::MIN_VERSION, control::Client, fake::FakeDaemon, system::SystemAction};
use rune::{Env, Runtime};
use serde_json::{Value, json};
use std::path::Path;
use std::sync::mpsc;

#[test]
fn selecting_tunnel_and_toggling_queues_the_correct_daemon_action() {
    let mut rt = Runtime::new(
        State::fixture("tunnels", "loaded").unwrap(),
        view,
        (1100.0, 760.0),
        Env::new(false, 1.0),
    );
    rt.frame(0.0);
    let (x, y) = rt.find("toggle-row-grafana").unwrap();
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

fn helper_reply(installed: bool) -> serde_json::Value {
    serde_json::json!({
        "entries": [],
        "helper": {"installed": installed, "entries": 0, "relay": {"enabled": false, "listening": false}},
    })
}

fn sync_request() -> reoclo_desktop::ui::Request {
    reoclo_desktop::ui::Request {
        method: "POST".into(),
        path: "/v1/hosts/sync".into(),
        body: None,
    }
}

#[test]
fn the_app_only_says_the_helper_is_installed_when_the_helper_says_so() {
    let mut state = State::fixture("hosts", "loaded").unwrap();
    state.awaiting_helper = true;
    state.receive(&sync_request(), Ok(helper_reply(false)));
    assert_eq!(state.notice, None);
    assert!(state.error.as_deref().unwrap().contains("not answering"));
    assert!(!state.awaiting_helper);

    let mut state = State::fixture("hosts", "loaded").unwrap();
    state.awaiting_helper = true;
    state.receive(&sync_request(), Ok(helper_reply(true)));
    assert_eq!(state.notice.as_deref(), Some("Hosts helper installed."));
    assert_eq!(state.error, None);
    assert!(state.model.status.hosts.unwrap().installed);
}

#[test]
fn a_routine_status_check_does_not_announce_an_install() {
    let mut state = State::fixture("hosts", "loaded").unwrap();
    state.receive(&sync_request(), Ok(helper_reply(true)));
    assert_eq!(state.notice, None);
}

#[test]
fn a_confirmation_goes_away_after_a_few_seconds() {
    let mut state = State::fixture("tunnels", "loaded").unwrap();
    let mut tx = rune::Tx::default();
    state.notice = Some("Endpoint copied.".into());
    assert!(!state.expire_notice(10.0, &mut tx));
    assert!(!state.expire_notice(13.0, &mut tx));
    assert!(state.notice.is_some());
    assert!(state.expire_notice(16.5, &mut tx));
    assert_eq!(state.notice, None);
}

#[test]
fn a_new_confirmation_gets_its_own_time_on_screen() {
    let mut state = State::fixture("tunnels", "loaded").unwrap();
    let mut tx = rune::Tx::default();
    state.notice = Some("Endpoint copied.".into());
    state.expire_notice(10.0, &mut tx);
    state.notice = Some("Tunnel stopped.".into());
    assert!(!state.expire_notice(15.0, &mut tx));
    assert!(!state.expire_notice(20.0, &mut tx));
    assert!(state.expire_notice(21.5, &mut tx));
}

#[test]
fn opening_hosts_asks_the_daemon_for_the_helper_status() {
    let mut rt = Runtime::new(
        State::fixture("tunnels", "loaded").unwrap(),
        view,
        (1100.0, 760.0),
        Env::new(false, 1.0),
    );
    rt.frame(0.0);
    let (x, y) = rt.find("nav-hosts").unwrap();
    rt.pointer_down(x, y);
    rt.pointer_up(x, y);
    rt.frame(0.02);
    let action = rt.state.outbox.pop().unwrap();
    assert_eq!(action.method, "GET");
    assert_eq!(action.path, "/v1/hosts");
}

#[test]
fn a_terminal_tunnel_shows_without_saved_tunnels_and_stops_after_confirming() {
    let mut rt = Runtime::new(
        State::fixture("tunnels", "terminal").unwrap(),
        view,
        (1100.0, 760.0),
        Env::new(false, 1.0),
    );
    rt.frame(0.0);
    let (x, y) = rt.find("stop-terminal-4242").unwrap();
    rt.pointer_down(x, y);
    rt.pointer_up(x, y);
    for _ in 0..30 {
        rt.frame(1.0 / 60.0);
    }
    assert!(rt.state.outbox.is_empty());
    let (x, y) = rt.find("confirm-action").unwrap();
    rt.pointer_down(x, y);
    rt.pointer_up(x, y);
    rt.frame(0.02);
    let action = rt.state.outbox.pop().unwrap();
    assert_eq!(action.method, "POST");
    assert_eq!(action.path, "/v1/foreground/4242/stop");
}

#[test]
fn an_older_daemon_without_terminal_tunnels_is_not_an_error() {
    use reoclo_desktop::ui::{Fault, Request};
    let mut state = State::fixture("tunnels", "loaded").unwrap();
    let request = Request {
        method: "GET".into(),
        path: "/v1/foreground".into(),
        body: None,
    };
    let fault = Fault::from(reoclo_desktop::control::Error::Api {
        status: 404,
        code: "not_found".into(),
        message: "no route for GET /v1/foreground".into(),
        state: None,
    });
    state.receive(&request, Err(fault));
    assert_eq!(state.error, None);
}

#[test]
fn the_terminal_tunnel_list_follows_daemon_events() {
    let mut state = State::fixture("tunnels", "loaded").unwrap();
    state
        .model
        .apply("foreground", r#"{"tunnels":[]}"#)
        .unwrap();
    assert!(state.model.foreground.is_empty());
}

fn click(rt: &mut Runtime<State>, key: &str) {
    let (x, y) = rt.find(key).unwrap();
    rt.pointer_down(x, y);
    rt.pointer_up(x, y);
    rt.frame(0.02);
}

/// Settings tall enough that the command-line tool section is on screen.
fn settings(state: State) -> Runtime<State> {
    let mut rt = Runtime::new(state, view, (1100.0, 1600.0), Env::new(false, 1.0));
    rt.frame(0.0);
    rt
}

#[test]
fn settings_shows_the_cli_and_offers_nothing_when_everything_is_current() {
    let rt = settings(State::fixture("settings", "loaded").unwrap());
    assert!(rt.find("cli-path").is_some());
    assert!(rt.find("restart-daemon").is_none());
    assert!(rt.find("update-helper").is_none());
}

#[test]
fn settings_restarts_a_daemon_older_than_the_cli_with_that_cli() {
    let mut state = State::fixture("settings", "outdated").unwrap();
    let (runner, actions) = mpsc::channel();
    state.system_runner = Some(runner);
    let mut rt = settings(state);
    assert!(rt.find("update-helper").is_some());
    click(&mut rt, "restart-daemon");
    let (action, cli) = actions.try_recv().unwrap();
    assert_eq!(action, SystemAction::RestartDaemon);
    assert_eq!(cli.as_deref(), Some(Path::new("/opt/homebrew/bin/reoclo")));
}

#[test]
fn a_cli_older_than_the_app_needs_blocks_actions_that_run_it() {
    let mut state = State::fixture("settings", "cli-old").unwrap();
    let (runner, actions) = mpsc::channel();
    state.system_runner = Some(runner);
    state.run_system(SystemAction::InstallDaemon);
    assert!(actions.try_recv().is_err());
    assert!(!state.busy);
    let error = state.error.clone().unwrap();
    assert!(error.contains("0.98.0"), "{error}");
    assert!(error.contains(MIN_VERSION), "{error}");
    assert!(error.contains("reoclo upgrade"), "{error}");
    state.run_system(SystemAction::StartAtLogin(true));
    assert_eq!(
        actions.try_recv().unwrap(),
        (SystemAction::StartAtLogin(true), None)
    );
}

#[test]
fn a_new_daemon_executable_or_opening_settings_searches_for_the_cli_again() {
    let mut state = State::fixture("tunnels", "loaded").unwrap();
    let (finder, searches) = mpsc::channel();
    state.cli_finder = Some(finder);
    let status = Request {
        method: "GET".into(),
        path: "/v1/status".into(),
        body: None,
    };
    let reply = || -> Result<Value, Fault> {
        Ok(json!({"version": "0.104.0", "executable": "/opt/homebrew/bin/reoclo"}))
    };
    state.receive(&status, reply());
    assert_eq!(
        searches.try_recv().unwrap().as_deref(),
        Some("/opt/homebrew/bin/reoclo")
    );
    state.receive(&status, reply());
    assert!(searches.try_recv().is_err());
    state.show_page(Page::Settings);
    assert_eq!(
        searches.try_recv().unwrap().as_deref(),
        Some("/opt/homebrew/bin/reoclo")
    );
}

#[test]
fn the_relay_switch_asks_first_and_keeps_trusted_domains() {
    let mut state = State::fixture("hosts", "loaded").unwrap();
    state.settings.ca_suffixes = vec!["internal".into(), "agents.localhost".into()];
    let mut rt = Runtime::new(state, view, (1100.0, 760.0), Env::new(false, 1.0));
    rt.frame(0.0);
    click(&mut rt, "relay-toggle");
    assert_eq!(rt.state.modal.as_deref(), Some("relay-on"));
    let setup = rt.state.helper_setup(true);
    assert_eq!(setup.relay_port, Some(8443));
    assert_eq!(setup.allow_suffixes, vec!["internal".to_owned()]);
    assert_eq!(rt.state.helper_setup(false).relay_port, None);
}

#[test]
fn removing_an_allowed_suffix_reissues_the_ca_without_it() {
    let mut state = State::fixture("certificates", "loaded").unwrap();
    state.settings.ca_suffixes = vec!["agents.localhost".into()];
    if let Some(ca) = &mut state.model.status.ca {
        ca.suffixes = vec!["agents.localhost".into(), "reoclo.localhost".into()];
    }
    let mut rt = Runtime::new(state, view, (1100.0, 760.0), Env::new(false, 1.0));
    rt.frame(0.0);
    assert!(rt.find("remove-suffix-reoclo.localhost").is_none());
    click(&mut rt, "remove-suffix-agents.localhost");
    for _ in 0..30 {
        rt.frame(1.0 / 60.0);
    }
    assert_eq!(rt.state.modal.as_deref(), Some("remove-suffix"));
    click(&mut rt, "confirm-action");
    let patch = rt.state.outbox.pop().unwrap();
    assert_eq!(patch.method, "PATCH");
    assert_eq!(
        patch.body.clone().unwrap()["ca_suffixes"],
        serde_json::json!([])
    );

    rt.state.receive(
        &patch,
        Ok(serde_json::json!({ "ca_suffixes": [], "manual_hosts": [] })),
    );
    assert_eq!(rt.state.modal, None);
    let trust = rt.state.outbox.pop().unwrap();
    assert_eq!(
        (trust.method.as_str(), trust.path.as_str()),
        ("POST", "/v1/ca/trust")
    );
    assert_eq!(trust.body, None);
    rt.state
        .receive(&trust, Ok(serde_json::json!({ "ok": true })));
    assert_eq!(
        rt.state.notice.as_deref(),
        Some("Domain removed and the new CA trusted.")
    );
}

#[test]
fn a_local_route_shows_this_mac_and_no_server_edit() {
    let mut state = State::fixture("tunnels", "loaded").unwrap();
    let local: reoclo_desktop::model::TunnelView = serde_json::from_value(serde_json::json!({
        "name": "dev-app", "org": "", "server": "", "server_id": "", "kind": "local",
        "state": "active", "enabled": true, "hostname": "app.reoclo.test",
        "url": "https://app.reoclo.test", "local_target": "127.0.0.1:43080"
    }))
    .unwrap();
    assert_eq!(local.place(), "This Mac");
    state.model.tunnels.push(local);
    state.model.selected = Some("dev-app".into());
    state.table.select("dev-app");
    let mut rt = Runtime::new(state, view, (1100.0, 760.0), Env::new(false, 1.0));
    rt.frame(0.0);
    assert!(rt.find("edit-tunnel").is_none());
    assert!(rt.find("remove-tunnel").is_some());
}

#[test]
fn certificates_can_trust_another_domain() {
    let mut state = State::fixture("certificates", "loaded").unwrap();
    state.trust_domain = rune::TextEdit::new(" Internal ");
    let mut rt = Runtime::new(state, view, (1100.0, 760.0), Env::new(false, 1.0));
    rt.frame(0.0);
    click(&mut rt, "trust-domain");
    let action = rt.state.outbox.pop().unwrap();
    assert_eq!(action.path, "/v1/ca/trust");
    assert_eq!(
        action.body.unwrap()["domains"],
        serde_json::json!(["internal"])
    );
}

#[test]
fn hosts_registers_and_removes_a_hostname() {
    let mut state = State::fixture("hosts", "loaded").unwrap();
    state.settings.manual_hosts = vec!["old.localhost".into()];
    state
        .model
        .host_entries
        .push(reoclo_desktop::model::HostEntry {
            hostname: "old.localhost".into(),
            source: "manual".into(),
        });
    state.new_host = rune::TextEdit::new("App.localhost");
    let mut rt = Runtime::new(state, view, (1100.0, 760.0), Env::new(false, 1.0));
    rt.frame(0.0);
    click(&mut rt, "register-host");
    let action = rt.state.outbox.pop().unwrap();
    assert_eq!(action.method, "PATCH");
    assert_eq!(
        action.body.unwrap()["manual_hosts"],
        serde_json::json!(["app.localhost", "old.localhost"])
    );
    rt.state.busy = false;
    rt.frame(0.02);
    click(&mut rt, "remove-host-old.localhost");
    let action = rt.state.outbox.pop().unwrap();
    assert_eq!(action.body.unwrap()["manual_hosts"], serde_json::json!([]));
}

#[test]
fn https_hostnames_sit_on_a_trusted_domain_picked_from_a_list() {
    let mut state = State::fixture("add", "loaded").unwrap();
    if let Some(ca) = &mut state.model.status.ca {
        ca.suffixes = vec!["agents.localhost".into(), "reoclo.localhost".into()];
    }
    let mut rt = Runtime::new(state, view, (1100.0, 760.0), Env::new(false, 1.0));
    rt.frame(0.0);
    click(&mut rt, "domain-trigger");
    for _ in 0..30 {
        rt.frame(1.0 / 60.0);
    }
    click(&mut rt, "domain-option-agents.localhost");
    assert_eq!(rt.state.domain, "agents.localhost");
    click(&mut rt, "save-tunnel");
    let action = rt.state.outbox.pop().unwrap();
    assert_eq!(action.body.unwrap()["hostname"], "api.agents.localhost");

    rt.state.busy = false;
    rt.state.hostname = rune::TextEdit::new("agents.localhost");
    rt.frame(0.02);
    click(&mut rt, "save-tunnel");
    let action = rt.state.outbox.pop().unwrap();
    assert_eq!(action.body.unwrap()["hostname"], "agents.localhost");
}

#[test]
fn without_a_trusted_ca_the_form_offers_to_trust_it_and_stays_open() {
    let mut state = State::fixture("add", "loaded").unwrap();
    if let Some(ca) = &mut state.model.status.ca {
        ca.trusted = false;
    }
    let mut rt = Runtime::new(state, view, (1100.0, 760.0), Env::new(false, 1.0));
    rt.frame(0.0);
    click(&mut rt, "trust-local-ca");
    let action = rt.state.outbox.pop().unwrap();
    assert_eq!(
        (action.path.as_str(), action.body.clone()),
        ("/v1/ca/trust", None)
    );
    rt.state
        .receive(&action, Ok(serde_json::json!({ "ok": true })));
    assert_eq!(rt.state.modal.as_deref(), Some("add"));
}

#[test]
fn saving_a_new_tunnel_sends_the_organization_tenant_id() {
    let mut state = State::fixture("add", "loaded").unwrap();
    state.orgs[0].id = "tenant-1".into();
    let mut rt = Runtime::new(state, view, (1100.0, 760.0), Env::new(false, 1.0));
    rt.frame(0.0);
    let (x, y) = rt.find("save-tunnel").unwrap();
    rt.pointer_down(x, y);
    rt.pointer_up(x, y);
    rt.frame(0.02);
    let action = rt.state.outbox.pop().unwrap();
    assert_eq!(action.path, "/v1/tunnels");
    assert_eq!(action.body.unwrap()["tenant_id"], "tenant-1");
}

#[test]
fn return_in_a_form_field_saves_the_tunnel() {
    let mut rt = Runtime::new(
        State::fixture("add", "loaded").unwrap(),
        view,
        (1100.0, 760.0),
        Env::new(false, 1.0),
    );
    rt.frame(0.0);
    let (x, y) = rt.find("tunnel-name").unwrap();
    rt.pointer_down(x, y);
    rt.pointer_up(x, y);
    rt.frame(0.02);
    assert!(rt.state.outbox.is_empty());
    rt.key(rune::KeyEvent {
        key: rune::Key::Enter,
        mods: Default::default(),
    });
    rt.frame(0.02);
    let action = rt.state.outbox.pop().unwrap();
    assert_eq!(action.method, "POST");
    assert_eq!(action.path, "/v1/tunnels");
}

#[test]
fn scrolling_the_form_reveals_hostname_without_moving_footer() {
    let mut state = State::fixture("add", "loaded").unwrap();
    state.via = "tailscale".into();
    state.error =
        Some("The daemon could not reach this server, so the tunnel was not saved.".into());
    let mut rt = Runtime::new(state, view, (1100.0, 760.0), Env::new(false, 1.0));
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
fn a_terminal_forward_can_be_given_an_https_address() {
    let mut rt = Runtime::new(
        State::fixture("tunnels", "terminal").unwrap(),
        view,
        (1100.0, 760.0),
        Env::new(false, 1.0),
    );
    rt.frame(0.0);
    let (x, y) = rt.find("https-terminal-4242-0").unwrap();
    rt.pointer_down(x, y);
    rt.pointer_up(x, y);
    for _ in 0..30 {
        rt.frame(1.0 / 60.0);
    }
    assert_eq!(rt.state.modal.as_deref(), Some("add"));
    assert_eq!(rt.state.kind, "https");
    assert_eq!(rt.state.target.text, "127.0.0.1:7700");
    assert_eq!(rt.state.name.text, "web-7700");

    let servers = reoclo_desktop::ui::Request {
        method: "GET".into(),
        path: "/v1/orgs/demo/servers".into(),
        body: None,
    };
    rt.state.receive(
        &servers,
        Ok(serde_json::json!({"servers": [{"id": "srv-9", "name": "Bastion", "slug": "bastion-1"}]})),
    );
    assert_eq!(rt.state.server, "srv-9");
    rt.frame(0.02);
    let (x, y) = rt.find("save-tunnel").unwrap();
    rt.pointer_down(x, y);
    rt.pointer_up(x, y);
    rt.frame(0.02);
    let action = rt.state.outbox.pop().unwrap();
    assert_eq!(action.path, "/v1/tunnels");
    let body = action.body.unwrap();
    assert_eq!(body["kind"], "https");
    assert_eq!(body["target"], "127.0.0.1:7700");
    assert_eq!(body["server_id"], "srv-9");
}

#[test]
fn a_helper_reinstall_while_the_helper_is_down_keeps_the_relay_it_had() {
    use reoclo_desktop::ui::read_preferences;
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("preferences.json");
    let mut state = State::fixture("hosts", "loaded").unwrap();
    state.preferences_path = Some(path.clone());
    let status = Request {
        method: "GET".into(),
        path: "/v1/status".into(),
        body: None,
    };
    state.receive(
        &status,
        Ok(json!({"version": "0.104.1", "hosts": {"installed": true, "version": "0.104.1", "entries": 8, "suffixes": ["reoclo.test"], "relay": {"enabled": true, "listening": true, "port": 8443}}})),
    );
    assert!(read_preferences(&path).relay);

    // A failed reinstall leaves the helper stopped, and a stopped helper reports no relay.
    state.receive(
        &status,
        Ok(json!({"version": "0.104.1", "hosts": {"installed": false, "entries": 8, "relay": {"enabled": false, "listening": false}}})),
    );
    assert!(state.relay_enabled());
    assert_eq!(
        state.helper_setup(state.relay_enabled()).relay_port,
        Some(state.settings.https_port)
    );
    assert!(read_preferences(&path).relay);
}

#[test]
fn a_running_helper_with_the_relay_off_is_remembered_too() {
    let mut state = State::fixture("hosts", "loaded").unwrap();
    state.relay_choice = true;
    state.receive(
        &Request {
            method: "GET".into(),
            path: "/v1/hosts".into(),
            body: None,
        },
        Ok(json!({"entries": [], "helper": {"installed": true, "entries": 0, "relay": {"enabled": false, "listening": false}}})),
    );
    assert!(!state.relay_choice);
    assert!(!state.relay_enabled());
}

#[test]
fn a_menu_bar_choice_is_remembered() {
    use reoclo_desktop::{model::MenuBarStyle, ui::read_preferences};
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("nested/preferences.json");
    assert_eq!(read_preferences(&path).menu_bar, MenuBarStyle::Icon);
    let mut state = State {
        preferences_path: Some(path.clone()),
        menu_bar: MenuBarStyle::IconAndMeter,
        ..State::default()
    };
    state.save_preferences();
    assert_eq!(state.error, None);
    assert_eq!(read_preferences(&path).menu_bar, MenuBarStyle::IconAndMeter);
}

#[test]
fn a_form_picker_can_be_searched_and_sets_its_value() {
    let mut rt = Runtime::new(
        State::fixture("add", "loaded").unwrap(),
        view,
        (1100.0, 760.0),
        Env::new(false, 1.0),
    );
    rt.frame(0.0);
    let (x, y) = rt.find("kind-trigger").unwrap();
    rt.pointer_down(x, y);
    rt.pointer_up(x, y);
    for _ in 0..30 {
        rt.frame(1.0 / 60.0);
    }
    assert!(rt.state.kind_picker.open);
    let (x, y) = rt.find("kind-option-TCP").unwrap();
    rt.pointer_down(x, y);
    rt.pointer_up(x, y);
    rt.frame(0.02);
    assert_eq!(rt.state.kind, "tcp");
    assert!(!rt.state.kind_picker.open);
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
