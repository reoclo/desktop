use reoclo_desktop::{
    agents::{Action, Change},
    cli::Cli,
    model::Preferences,
    system::{
        SystemAction,
        command_path::{CommandPaths, CommandState},
    },
    ui::{
        AgentsStep, DAEMON_WAIT_SECONDS, FirstRun, LoginStep, Request, State, ViewState,
        read_preferences, view,
    },
};
use rune::{Env, Runtime};
use serde_json::json;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver};

const BUNDLED: &str = "/Applications/Reoclo.app/Contents/Resources/reoclo";
const LINK: &str = "/Users/example/.local/bin/reoclo";
const HTTPS_GRAFANA: &str = "reoclo://open?v=1&org=demo&server=prod-1&port=3000&kind=https";
const LISTED: &str = r#"{"agents":[
  {"id":"claude-code","name":"Claude Code","installed":true,"config_path":"/Users/example/.claude.json","registered":false,"command":null,"error":null},
  {"id":"codex","name":"Codex","installed":true,"config_path":"/Users/example/.codex/config.toml","registered":false,"command":null,"error":null}]}"#;

type Actions = Receiver<(SystemAction, Option<PathBuf>)>;

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

fn asked(state: &State) -> Vec<String> {
    state
        .outbox
        .iter()
        .map(|r| format!("{} {}", r.method, r.path))
        .collect()
}

fn request(method: &str, path: &str) -> Request {
    Request {
        method: method.into(),
        path: path.into(),
        body: None,
    }
}

fn paths() -> CommandPaths {
    CommandPaths {
        home: "/Users/example".into(),
        bundled: BUNDLED.into(),
    }
}

/// A first launch on a Mac with only the app on it: the daemon is not running.
fn fresh() -> (State, Actions) {
    let (runner, actions) = mpsc::channel();
    let mut state = State::launch(&Preferences::default());
    state.phase = ViewState::Offline;
    state.cli = Some(Cli::Ready {
        path: BUNDLED.into(),
        version: "0.117.0".into(),
        bundled: true,
    });
    state.command_paths = Some(paths());
    state.system_runner = Some(runner);
    (state, actions)
}

/// The daemon runs and answers: signed in or not, with the certificate and helper not yet set up.
fn running(signed_in: bool) -> (State, Actions) {
    let (mut state, actions) = fresh();
    state.phase = ViewState::Loaded;
    state.service_installed = true;
    state.model.status.version = "0.117.0".into();
    state.model.status.signed_in = Some(signed_in);
    (state, actions)
}

fn trust_certificate(state: &mut State) {
    state.model.status.ca = serde_json::from_value(json!({"exists": true, "trusted": true})).ok();
}

fn install_helper(state: &mut State) {
    state.model.status.hosts = serde_json::from_value(json!({"installed": true})).ok();
}

fn at_ready() -> (State, Actions) {
    let (mut state, actions) = running(true);
    trust_certificate(&mut state);
    install_helper(&mut state);
    state.finish_tools();
    (state, actions)
}

#[test]
fn first_launch_routes_to_first_run_and_a_finished_one_does_not() {
    assert!(State::launch(&Preferences::default()).first_run);
    let finished = Preferences {
        onboarded: true,
        ..Preferences::default()
    };
    assert!(!State::launch(&finished).first_run);

    let loading = runtime(State::launch(&Preferences::default()));
    assert!(loading.find("loading-state").is_some());
    let (state, _) = fresh();
    let rt = runtime(state);
    assert!(rt.find("first-run").is_some());
    assert!(rt.find("first-run-next").is_some());
}

#[test]
fn each_step_follows_the_plumbing_state() {
    let (mut state, actions) = fresh();
    assert_eq!(state.first_run_step(), FirstRun::Welcome);

    state.begin_login();
    assert_eq!(*state.login_step(), LoginStep::InstallingService);
    assert_eq!(state.first_run_step(), FirstRun::Waiting);
    let (action, _) = actions.try_recv().unwrap();
    state.system_finished(&action, Err("launchctl failed".into()));
    assert_eq!(state.first_run_step(), FirstRun::Welcome);

    let (mut state, _actions) = running(false);
    assert_eq!(state.first_run_step(), FirstRun::Welcome);
    state.begin_login();
    state.apply_event(
        "auth",
        r#"{"status":"pending","user_code":"KQTM-7294","verification_uri":"https://auth.reoclo.com/device"}"#,
    );
    assert_eq!(state.first_run_step(), FirstRun::Waiting);
    state.apply_event("auth", r#"{"status":"success"}"#);
    assert_eq!(state.first_run_step(), FirstRun::Pairing);

    trust_certificate(&mut state);
    assert_eq!(state.first_run_step(), FirstRun::PairingHelper);
    install_helper(&mut state);
    assert_eq!(state.first_run_step(), FirstRun::Tools);

    state.cli = Some(Cli::Ready {
        path: "/opt/homebrew/bin/reoclo".into(),
        version: "0.117.0".into(),
        bundled: false,
    });
    assert_eq!(state.first_run_step(), FirstRun::ToolsBrew);
    state.finish_tools();
    assert_eq!(state.first_run_step(), FirstRun::Ready);

    let (mut state, _actions) = running(true);
    trust_certificate(&mut state);
    install_helper(&mut state);
    state.command = Some(CommandState::Installed { link: LINK.into() });
    assert_eq!(state.first_run_step(), FirstRun::Ready);
}

#[test]
fn setting_up_later_moves_on_to_the_tools() {
    let (mut state, _actions) = running(true);
    assert_eq!(state.first_run_step(), FirstRun::Pairing);
    state.skip_pairing();
    assert_eq!(state.first_run_step(), FirstRun::Tools);
}

#[test]
fn a_macos_prompt_is_explained_on_screen_before_continue_raises_it() {
    let (state, actions) = running(true);
    let mut rt = runtime(state);
    assert!(rt.find("first-run-prompt-notice").is_some());
    assert!(rt.state.outbox.is_empty());

    click(&mut rt, "first-run-next");
    assert_eq!(asked(&rt.state), ["POST /v1/ca/trust"]);
    rt.state
        .receive(&request("POST", "/v1/ca/trust"), Ok(json!({"ok": true})));
    let mut rt = runtime(rt.state);
    assert_eq!(rt.state.first_run_step(), FirstRun::PairingHelper);
    assert!(rt.find("first-run-prompt-notice").is_some());
    assert!(actions.try_recv().is_err());

    click(&mut rt, "first-run-next");
    assert!(matches!(
        actions.try_recv().unwrap().0,
        SystemAction::SetupHosts(_)
    ));
}

#[test]
fn a_service_that_never_answers_times_out_and_log_in_tries_again() {
    let (mut state, actions) = fresh();
    let mut tx = rune::Tx::default();
    state.begin_login();
    let (install, _) = actions.try_recv().unwrap();
    state.system_finished(&install, Ok("installed".into()));
    assert_eq!(*state.login_step(), LoginStep::WaitingForDaemon);

    state.poll(10.0, &mut tx);
    state.poll(10.0 + DAEMON_WAIT_SECONDS - 1.0, &mut tx);
    assert_eq!(*state.login_step(), LoginStep::WaitingForDaemon);
    assert!(state.poll(10.0 + DAEMON_WAIT_SECONDS + 1.0, &mut tx));
    let LoginStep::Failed(message) = state.login_step() else {
        panic!("expected the wait to fail, got {:?}", state.login_step());
    };
    assert!(message.contains("did not start"), "{message}");
    assert_eq!(state.first_run_step(), FirstRun::Welcome);

    state.begin_login();
    assert_eq!(*state.login_step(), LoginStep::InstallingService);
    assert_eq!(actions.try_recv().unwrap().0, SystemAction::InstallDaemon);
}

#[test]
fn a_service_that_answers_in_time_has_no_deadline() {
    let (mut state, actions) = fresh();
    let mut tx = rune::Tx::default();
    state.begin_login();
    let (install, _) = actions.try_recv().unwrap();
    state.system_finished(&install, Ok("installed".into()));
    state.poll(10.0, &mut tx);
    state.receive(
        &request("GET", "/v1/status"),
        Ok(json!({"version": "0.117.0", "signed_in": false})),
    );
    assert_eq!(*state.login_step(), LoginStep::RequestingCode);
    state.poll(10.0 + DAEMON_WAIT_SECONDS * 4.0, &mut tx);
    assert_eq!(*state.login_step(), LoginStep::RequestingCode);
}

#[test]
fn the_ready_screen_lists_the_agents_the_cli_reports() {
    let (mut state, actions) = at_ready();
    let mut tx = rune::Tx::default();
    state.poll(0.0, &mut tx);
    assert_eq!(state.agents, AgentsStep::Checking);
    let (action, _) = actions.try_recv().unwrap();
    assert_eq!(
        action,
        SystemAction::ListAgents {
            program: BUNDLED.into()
        }
    );
    state.poll(1.0, &mut tx);
    assert!(actions.try_recv().is_err());

    state.system_finished(&action, Ok(LISTED.into()));
    let AgentsStep::Listed(found) = &state.agents else {
        panic!("expected the agents, got {:?}", state.agents);
    };
    assert_eq!(found.len(), 2);
    assert_eq!(found[0].name, "Claude Code");
    assert_eq!(found[1].registered, Some(false));
    assert_eq!(state.first_run_step(), FirstRun::ReadyAgents);
}

#[test]
fn a_cli_that_cannot_list_agents_leaves_the_ready_screen_without_the_section() {
    for answer in [
        Err("unknown command 'agents'".to_owned()),
        Ok("not json".into()),
    ] {
        let (mut state, actions) = at_ready();
        state.poll(0.0, &mut rune::Tx::default());
        let (action, _) = actions.try_recv().unwrap();
        state.system_finished(&action, answer);
        assert!(matches!(state.agents, AgentsStep::Unavailable(_)));
        assert_eq!(state.first_run_step(), FirstRun::Ready);
    }
}

#[test]
fn a_mac_with_no_agents_has_no_connect_section() {
    let (mut state, actions) = at_ready();
    state.poll(0.0, &mut rune::Tx::default());
    let (action, _) = actions.try_recv().unwrap();
    let none = r#"{"agents":[{"id":"codex","name":"Codex","installed":false,"config_path":"/Users/example/.codex/config.toml","registered":false,"command":null,"error":null}]}"#;
    state.system_finished(&action, Ok(none.into()));
    assert_eq!(state.first_run_step(), FirstRun::Ready);
}

fn listed() -> (State, Actions) {
    let (mut state, actions) = at_ready();
    state.poll(0.0, &mut rune::Tx::default());
    let (action, _) = actions.try_recv().unwrap();
    state.system_finished(&action, Ok(LISTED.into()));
    (state, actions)
}

#[test]
fn connect_registers_the_path_link_and_reports_each_result() {
    let (mut state, actions) = listed();
    state.command = Some(CommandState::Installed { link: LINK.into() });
    let mut rt = runtime(state);
    click(&mut rt, "first-run-connect-agents");
    let (action, cli) = actions.try_recv().unwrap();
    assert_eq!(
        action,
        SystemAction::ConnectAgents {
            program: BUNDLED.into(),
            command: LINK.into()
        }
    );
    assert!(cli.is_none());
    assert!(matches!(rt.state.agents, AgentsStep::Connecting(_)));

    let results = r#"{"results":[
      {"agent":"claude-code","name":"Claude Code","action":"installed","config_path":"/Users/example/.claude.json","backup_path":null,"command":"/Users/example/.local/bin/reoclo","error":null},
      {"agent":"codex","name":"Codex","action":"failed","config_path":"/Users/example/.codex/config.toml","backup_path":null,"command":null,"error":{"kind":"parse","message":"config.toml is not valid TOML"}}]}"#;
    rt.state.system_finished(&action, Ok(results.into()));
    let AgentsStep::Done { results, .. } = &rt.state.agents else {
        panic!("expected results, got {:?}", rt.state.agents);
    };
    assert_eq!(results.len(), 2);
    assert_eq!(results[0].action, Action::Installed);
    let Change { name, error, .. } = &results[1];
    assert_eq!(name, "Codex");
    assert_eq!(
        error.as_ref().map(|e| e.message.as_str()),
        Some("config.toml is not valid TOML")
    );
}

#[test]
fn connect_registers_the_bundled_binary_when_the_app_has_no_command_on_path() {
    let (mut state, actions) = listed();
    state.command = None;
    let mut rt = runtime(state);
    click(&mut rt, "first-run-connect-agents");
    let SystemAction::ConnectAgents { command, .. } = actions.try_recv().unwrap().0 else {
        panic!("expected the connect action");
    };
    assert_eq!(command, Path::new(BUNDLED));
}

#[test]
fn connect_registers_the_existing_command_that_stays_in_charge() {
    let (mut state, actions) = listed();
    state.command = Some(CommandState::AlreadyPresent {
        path: "/opt/homebrew/bin/reoclo".into(),
        version: None,
    });
    let mut rt = runtime(state);
    click(&mut rt, "first-run-connect-agents");
    let (action, _) = actions.try_recv().unwrap();
    assert_eq!(
        action,
        SystemAction::ConnectAgents {
            program: BUNDLED.into(),
            command: "/opt/homebrew/bin/reoclo".into()
        }
    );
}

#[test]
fn a_connect_that_cannot_run_reports_why() {
    let (mut state, actions) = listed();
    let mut rt = runtime({
        state.command = None;
        state
    });
    click(&mut rt, "first-run-connect-agents");
    let (action, _) = actions.try_recv().unwrap();
    rt.state
        .system_finished(&action, Err("The reoclo CLI could not start.".into()));
    assert_eq!(
        rt.state.agents,
        AgentsStep::Failed {
            agents: rt.state.agents.found().to_vec(),
            message: "The reoclo CLI could not start.".into()
        }
    );
    assert!(rt.state.agents.found().len() == 2);
}

fn link_state() -> (State, Actions) {
    let (mut state, actions) = fresh();
    state.open_link(HTTPS_GRAFANA);
    (state, actions)
}

#[test]
fn a_link_that_arrives_during_first_run_is_kept_and_shown() {
    let (state, _actions) = link_state();
    assert!(state.link.is_some());
    assert_eq!(state.modal, None);
    assert_eq!(state.first_run_step(), FirstRun::Link);
    let rt = runtime(state);
    assert!(rt.find("first-run-next").is_some());
}

#[test]
fn a_kept_link_opens_once_done_is_pressed() {
    let (mut state, _actions) = link_state();
    state.phase = ViewState::Loaded;
    state.service_installed = true;
    state.model.status.version = "0.117.0".into();
    state.model.status.signed_in = Some(true);
    trust_certificate(&mut state);
    install_helper(&mut state);
    state.finish_tools();
    assert_eq!(state.first_run_step(), FirstRun::Ready);
    assert!(state.outbox.is_empty());

    let mut rt = runtime(state);
    click(&mut rt, "first-run-next");
    assert!(!rt.state.first_run);
    assert_eq!(asked(&rt.state), ["GET /v1/orgs"]);
    assert!(rt.state.link.is_some());
}

#[test]
fn cancelling_the_kept_link_drops_it() {
    let (state, _actions) = link_state();
    let mut rt = runtime(state);
    click(&mut rt, "first-run-cancel");
    assert!(rt.state.link.is_none());
    assert_eq!(rt.state.first_run_step(), FirstRun::Welcome);
}

#[test]
fn only_done_sets_the_onboarded_preference() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("preferences.json");
    let (mut state, _actions) = running(true);
    state.preferences_path = Some(path.clone());
    state.skip_pairing();
    state.finish_tools();
    assert!(!state.onboarded);
    assert!(!read_preferences(&path).onboarded);

    let mut rt = runtime(state);
    assert_eq!(rt.state.first_run_step(), FirstRun::Ready);
    click(&mut rt, "first-run-next");
    assert!(rt.state.onboarded);
    assert!(!rt.state.first_run);
    assert!(read_preferences(&path).onboarded);
}

#[test]
fn run_setup_again_opens_first_run_with_a_fresh_flow() {
    let (mut state, _actions) = at_ready();
    state.onboarded = true;
    state.first_run = false;
    state.begin_setup();
    assert!(state.first_run);
    assert_eq!(asked(&state), ["GET /v1/status"]);
    assert_eq!(state.first_run_step(), FirstRun::Tools);
}
