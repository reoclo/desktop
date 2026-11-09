use reoclo_desktop::system::{FakeRunner, SystemAction, SystemRunner, admin_script, launch_agent};

#[test]
fn injected_runner_never_changes_system_state() {
    let runner = FakeRunner;
    for action in [
        SystemAction::StartDaemon,
        SystemAction::SetupHosts,
        SystemAction::StartAtLogin(true),
    ] {
        assert!(runner.run(&action).unwrap().contains("no system changes"));
    }
}

#[test]
fn administrator_command_quotes_an_application_path_with_spaces_and_quotes() {
    assert_eq!(
        admin_script("/tmp/Reoclo's App/reoclo"),
        "do shell script \"'/tmp/Reoclo'\\\\''s App/reoclo' hosts setup\" with administrator privileges"
    );
}

#[test]
fn launch_agent_escapes_xml_paths() {
    let plist = launch_agent("/tmp/A&B/Reoclo");
    assert!(plist.contains("<string>/tmp/A&amp;B/Reoclo</string>"));
    assert!(plist.contains("<key>RunAtLoad</key><true/>"));
}
