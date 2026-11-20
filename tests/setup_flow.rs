use reoclo_desktop::{
    cli::Cli,
    system::SystemAction,
    ui::{Effect, LoginStep, Request, SetupStep, State, StepState, ViewState},
};
use serde_json::{Value, json};
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver};

const BUNDLED: &str = "/Applications/Reoclo.app/Contents/Resources/reoclo";
const PENDING: &str = r#"{"status":"pending","user_code":"KQTM-7294","verification_uri":"https://auth.reoclo.com/device","verification_uri_complete":"https://auth.reoclo.com/device?user_code=KQTM-7294"}"#;

type Actions = Receiver<(SystemAction, Option<PathBuf>)>;

/// A Mac with only the app on it: no CLI config, no daemon, and the bundled CLI to run.
fn nothing_installed() -> (State, Actions) {
    let (runner, actions) = mpsc::channel();
    let state = State {
        phase: ViewState::Offline,
        cli: Some(Cli::Ready {
            path: BUNDLED.into(),
            version: "0.116.2".into(),
            bundled: true,
        }),
        system_runner: Some(runner),
        ..State::default()
    };
    (state, actions)
}

fn request(method: &str, path: &str) -> Request {
    Request {
        method: method.into(),
        path: path.into(),
        body: None,
    }
}

fn asked(state: &State) -> Vec<String> {
    state
        .outbox
        .iter()
        .map(|r| format!("{} {}", r.method, r.path))
        .collect()
}

fn status(version: &str) -> Value {
    json!({"version": version, "signed_in": false})
}

#[test]
fn logging_in_with_nothing_installed_starts_the_service_then_signs_in_through_the_daemon() {
    let (mut state, actions) = nothing_installed();
    state.begin_login();
    assert_eq!(*state.login_step(), LoginStep::InstallingService);
    let (action, cli) = actions.try_recv().unwrap();
    assert_eq!(action, SystemAction::InstallDaemon);
    assert_eq!(cli.as_deref(), Some(std::path::Path::new(BUNDLED)));
    assert!(state.outbox.is_empty());

    state.system_finished(&action, Ok("installed".into()));
    assert_eq!(*state.login_step(), LoginStep::WaitingForDaemon);
    assert!(!asked(&state).contains(&"POST /v1/auth/login".to_owned()));

    state.receive(&request("GET", "/v1/status"), Ok(status("0.116.2")));
    assert_eq!(*state.login_step(), LoginStep::RequestingCode);
    assert_eq!(asked(&state).last().unwrap(), "POST /v1/auth/login");

    state.receive(&request("POST", "/v1/auth/login"), Ok(json!({"ok": true})));
    state.apply_event("auth", PENDING);
    assert_eq!(*state.login_step(), LoginStep::WaitingForApproval);
    assert_eq!(state.model.auth.user_code.as_deref(), Some("KQTM-7294"));
    assert_eq!(
        state.take_effects(),
        [Effect::OpenUrl(
            "https://auth.reoclo.com/device?user_code=KQTM-7294".into()
        )]
    );

    state.apply_event("auth", PENDING);
    assert!(state.take_effects().is_empty());

    state.apply_event("auth", r#"{"status":"success"}"#);
    assert_eq!(*state.login_step(), LoginStep::SignedIn);
    assert!(state.signed_in());
}

#[test]
fn a_running_daemon_skips_the_service_install() {
    let (mut state, actions) = nothing_installed();
    state.phase = ViewState::Empty;
    state.begin_login();
    assert!(actions.try_recv().is_err());
    assert_eq!(*state.login_step(), LoginStep::RequestingCode);
    assert_eq!(asked(&state), ["POST /v1/auth/login"]);
}

#[test]
fn a_service_that_will_not_install_fails_the_login_with_the_reason() {
    let (mut state, actions) = nothing_installed();
    state.begin_login();
    let (action, _) = actions.try_recv().unwrap();
    state.system_finished(&action, Err("launchctl bootstrap failed".into()));
    assert_eq!(
        *state.login_step(),
        LoginStep::Failed("launchctl bootstrap failed".into())
    );
    assert_eq!(state.error.as_deref(), Some("launchctl bootstrap failed"));
}

#[test]
fn a_refused_sign_in_fails_the_login_and_a_second_try_starts_over() {
    let (mut state, _actions) = nothing_installed();
    state.phase = ViewState::Empty;
    state.begin_login();
    state.apply_event("auth", PENDING);
    state.apply_event(
        "auth",
        r#"{"status":"failed","message":"The code expired."}"#,
    );
    assert_eq!(
        *state.login_step(),
        LoginStep::Failed("The code expired.".into())
    );
    state.outbox.clear();
    state.pending.clear();
    state.begin_login();
    assert_eq!(*state.login_step(), LoginStep::RequestingCode);
    assert_eq!(asked(&state), ["POST /v1/auth/login"]);
}

/// The service is running and signed in; the certificate and the hosts helper are not set up.
fn needs_pairing() -> (State, Actions) {
    let mut state = State::fixture("first-run", "pairing").unwrap();
    let (runner, actions) = mpsc::channel();
    state.system_runner = Some(runner);
    state.busy = false;
    state.model.status.hosts = None;
    state.outbox.clear();
    state.pending.clear();
    (state, actions)
}

#[test]
fn each_macos_prompt_waits_for_an_explanation_and_a_continue() {
    let (mut state, actions) = needs_pairing();
    state.start_setup();
    assert_eq!(state.setup_step(SetupStep::Service), StepState::Done);
    assert_eq!(
        state.setup_step(SetupStep::Certificate),
        StepState::Explaining
    );
    assert_eq!(state.setup_step(SetupStep::Helper), StepState::Needed);
    assert!(state.outbox.is_empty());

    state.continue_setup();
    assert_eq!(state.setup_step(SetupStep::Certificate), StepState::Running);
    assert_eq!(asked(&state), ["POST /v1/ca/trust"]);

    state.receive(&request("POST", "/v1/ca/trust"), Ok(json!({"ok": true})));
    assert_eq!(state.setup_step(SetupStep::Certificate), StepState::Done);
    assert_eq!(state.setup_step(SetupStep::Helper), StepState::Explaining);
    assert!(actions.try_recv().is_err());

    state.continue_setup();
    assert_eq!(state.setup_step(SetupStep::Helper), StepState::Running);
    let (action, cli) = actions.try_recv().unwrap();
    assert!(cli.is_some());
    let SystemAction::SetupHosts(setup) = &action else {
        panic!("expected the hosts helper install, got {action:?}");
    };
    assert_eq!(setup.relay_port, Some(state.settings.https_port));

    state.system_finished(&action, Ok("Hosts helper installed.".into()));
    state.receive(
        &request("POST", "/v1/hosts/sync"),
        Ok(json!({"entries": [], "helper": {"installed": true, "entries": 0, "relay": {"enabled": true, "listening": true}}})),
    );
    assert_eq!(state.setup_step(SetupStep::Helper), StepState::Done);
    assert!(state.secure_ready());
}

#[test]
fn the_relay_is_on_even_when_the_last_helper_had_it_off() {
    let (mut state, actions) = needs_pairing();
    state.relay_choice = false;
    state.start_setup();
    state.continue_setup();
    state.receive(&request("POST", "/v1/ca/trust"), Ok(json!({"ok": true})));
    state.continue_setup();
    let (SystemAction::SetupHosts(setup), _) = actions.try_recv().unwrap() else {
        panic!("expected the hosts helper install");
    };
    assert!(setup.relay_port.is_some());
}

#[test]
fn canceling_the_administrator_prompt_returns_to_the_explanation_with_the_reason() {
    let (mut state, actions) = needs_pairing();
    state.start_setup();
    state.continue_setup();
    state.receive(&request("POST", "/v1/ca/trust"), Ok(json!({"ok": true})));
    state.continue_setup();
    let (action, _) = actions.try_recv().unwrap();
    state.system_finished(&action, Err("Administrator approval was canceled.".into()));
    assert_eq!(
        state.setup_step(SetupStep::Helper),
        StepState::Failed("Administrator approval was canceled.".into())
    );
    state.continue_setup();
    assert!(matches!(
        actions.try_recv().unwrap().0,
        SystemAction::SetupHosts(_)
    ));
}

#[test]
fn a_certificate_the_person_declines_to_trust_reports_the_failure() {
    let (mut state, _actions) = needs_pairing();
    state.start_setup();
    state.continue_setup();
    state.receive(
        &request("POST", "/v1/ca/trust"),
        Err(reoclo_desktop::ui::Fault::Error(
            "The trust change was canceled.".into(),
        )),
    );
    assert_eq!(
        state.setup_step(SetupStep::Certificate),
        StepState::Failed("The trust change was canceled.".into())
    );
}

/// What a daemon older than the idle-timeout fix sends: it trusts the CA, then closes the socket without a reply.
fn lost_reply() -> reoclo_desktop::ui::Fault {
    reoclo_desktop::ui::Fault::from(reoclo_desktop::control::Error::Protocol(
        "Missing HTTP status".into(),
    ))
}

fn status_with_ca(trusted: bool) -> Value {
    json!({"version": "0.116.2", "signed_in": true, "ca": {"exists": true, "trusted": trusted, "suffixes": [], "stale": false}})
}

#[test]
fn a_trust_reply_that_is_lost_while_the_ca_ends_up_trusted_finishes_the_step() {
    let (mut state, _actions) = needs_pairing();
    state.start_setup();
    state.continue_setup();
    state.receive(&request("POST", "/v1/ca/trust"), Err(lost_reply()));
    assert_eq!(asked(&state).last().unwrap(), "GET /v1/status");
    assert_eq!(state.setup_step(SetupStep::Certificate), StepState::Running);

    state.receive(&request("GET", "/v1/status"), Ok(status_with_ca(true)));
    assert_eq!(state.setup_step(SetupStep::Certificate), StepState::Done);
    assert_eq!(state.setup_step(SetupStep::Helper), StepState::Explaining);
    assert_eq!(state.error, None);
}

#[test]
fn a_trust_reply_that_is_lost_while_the_ca_stays_untrusted_reports_the_failure() {
    let (mut state, _actions) = needs_pairing();
    state.start_setup();
    state.continue_setup();
    state.receive(&request("POST", "/v1/ca/trust"), Err(lost_reply()));
    state.receive(&request("GET", "/v1/status"), Ok(status_with_ca(false)));
    let StepState::Failed(message) = state.setup_step(SetupStep::Certificate) else {
        panic!("the certificate step should have failed");
    };
    assert!(message.contains("Missing HTTP status"), "{message}");
    assert_eq!(state.error.as_deref(), Some(message.as_str()));
}

#[test]
fn a_trust_reply_that_is_lost_with_no_answer_to_the_status_check_reports_the_failure() {
    let (mut state, _actions) = needs_pairing();
    state.start_setup();
    state.continue_setup();
    state.receive(&request("POST", "/v1/ca/trust"), Err(lost_reply()));
    state.receive(
        &request("GET", "/v1/status"),
        Err(reoclo_desktop::ui::Fault::Error("no answer".into())),
    );
    let StepState::Failed(message) = state.setup_step(SetupStep::Certificate) else {
        panic!("the certificate step should have failed");
    };
    assert!(message.contains("Missing HTTP status"), "{message}");
}

#[test]
fn setup_starts_the_service_first_when_it_is_not_running() {
    let (mut state, actions) = nothing_installed();
    state.start_setup();
    assert_eq!(state.setup_step(SetupStep::Service), StepState::Running);
    let (action, _) = actions.try_recv().unwrap();
    assert_eq!(action, SystemAction::InstallDaemon);
    state.system_finished(&action, Ok("installed".into()));
    state.receive(&request("GET", "/v1/status"), Ok(status("0.116.2")));
    state.receive(&request("GET", "/v1/tunnels"), Ok(json!({"tunnels": []})));
    assert_eq!(state.setup_step(SetupStep::Service), StepState::Done);
    assert_eq!(
        state.setup_step(SetupStep::Certificate),
        StepState::Explaining
    );
}

#[test]
fn an_app_update_leaves_a_helper_that_still_runs_the_old_copy_flagged_for_reinstall() {
    let (mut state, _actions) = nothing_installed();
    state.receive(
        &request("GET", "/v1/status"),
        Ok(json!({
            "version": "0.116.1",
            "hosts": {"installed": true, "version": "0.116.1"}
        })),
    );
    assert_eq!(state.helper_behind().as_deref(), Some("0.116.1"));
}

#[test]
fn the_tools_step_hands_the_command_install_and_removal_to_the_worker() {
    use reoclo_desktop::system::worker::CommandRequest;
    let (mut state, _actions) = nothing_installed();
    let (worker, requests) = mpsc::channel();
    state.command_worker = Some(worker);
    state.install_command();
    state.remove_command();
    assert_eq!(requests.try_recv().unwrap(), CommandRequest::Install);
    assert_eq!(requests.try_recv().unwrap(), CommandRequest::Remove);
}
