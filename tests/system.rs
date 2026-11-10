use reoclo_desktop::system::{
    FakeRunner, HelperSetup, MacRunner, SystemAction, SystemRunner, admin_error, admin_script,
    launch_agent,
};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

#[test]
fn injected_runner_never_changes_system_state() {
    let runner = FakeRunner;
    for action in [
        SystemAction::InstallDaemon,
        SystemAction::RestartDaemon,
        SystemAction::SetupHosts(Default::default()),
        SystemAction::StartAtLogin(true),
    ] {
        assert!(
            runner
                .run(&action, None)
                .unwrap()
                .contains("no system changes")
        );
    }
}

/// A stand-in CLI that records each command line it is given.
fn recording_cli(dir: &Path) -> PathBuf {
    let cli = dir.join("reoclo");
    let calls = dir.join("calls");
    std::fs::write(
        &cli,
        format!("#!/bin/sh\necho \"$@\" >> '{}'\n", calls.display()),
    )
    .unwrap();
    std::fs::set_permissions(&cli, std::fs::Permissions::from_mode(0o755)).unwrap();
    cli
}

fn mac_runner(dir: &Path) -> MacRunner {
    MacRunner {
        desktop: dir.join("Reoclo"),
        launch_agent: dir.join("com.reoclo.desktop.plist"),
        service_plist: dir.join("com.reoclo.tunneld.plist"),
    }
}

#[test]
fn restart_stops_the_daemon_then_starts_it_under_launchd_when_the_service_is_installed() {
    let dir = tempfile::tempdir().unwrap();
    let cli = recording_cli(dir.path());
    let runner = mac_runner(dir.path());
    assert_eq!(
        runner
            .run(&SystemAction::RestartDaemon, Some(&cli))
            .unwrap(),
        "Daemon restarted."
    );
    std::fs::write(&runner.service_plist, "").unwrap();
    runner
        .run(&SystemAction::RestartDaemon, Some(&cli))
        .unwrap();
    assert_eq!(
        std::fs::read_to_string(dir.path().join("calls")).unwrap(),
        "tunnel daemon stop\ntunnel daemon start\ntunnel daemon stop\ntunnel daemon install\n"
    );
}

#[test]
fn actions_that_need_the_cli_refuse_to_run_without_one() {
    let dir = tempfile::tempdir().unwrap();
    let runner = mac_runner(dir.path());
    for action in [SystemAction::InstallDaemon, SystemAction::RestartDaemon] {
        assert!(
            runner
                .run(&action, None)
                .unwrap_err()
                .contains("No reoclo CLI")
        );
    }
}

#[test]
fn administrator_command_quotes_an_application_path_with_spaces_and_quotes() {
    assert_eq!(
        admin_script("/tmp/Reoclo's App/reoclo", 501, &Default::default()),
        "do shell script \"'/tmp/Reoclo'\\\\''s App/reoclo' __hostsd-install --uid 501\" with administrator privileges"
    );
}

#[test]
fn administrator_errors_lose_the_script_location_and_code() {
    assert_eq!(
        admin_error("0:121: execution error: hosts: something failed (2)\n"),
        "hosts: something failed"
    );
    assert_eq!(
        admin_error("0:121: execution error: User canceled. (-128)"),
        "Administrator approval was canceled."
    );
    assert_eq!(admin_error("plain failure"), "plain failure");
}

#[test]
fn launch_agent_escapes_xml_paths() {
    let plist = launch_agent("/tmp/A&B/Reoclo");
    assert!(plist.contains("<string>/tmp/A&amp;B/Reoclo</string>"));
    assert!(plist.contains("<key>RunAtLoad</key><true/>"));
}

#[test]
fn the_helper_install_carries_the_relay_and_extra_suffixes() {
    let script = admin_script(
        "/Applications/Reoclo.app/Contents/Resources/reoclo",
        501,
        &HelperSetup {
            relay_port: Some(8443),
            allow_suffixes: vec!["internal".into()],
        },
    );
    assert!(script.contains(
        "__hostsd-install --uid 501 --allow-suffix 'internal' --port-443 --relay-port 8443"
    ));
}
