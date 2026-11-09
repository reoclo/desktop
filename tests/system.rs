use reoclo_desktop::system::{
    FakeRunner, HelperSetup, SystemAction, SystemRunner, admin_error, admin_script, launch_agent,
};

#[test]
fn injected_runner_never_changes_system_state() {
    let runner = FakeRunner;
    for action in [
        SystemAction::StartDaemon,
        SystemAction::SetupHosts(Default::default()),
        SystemAction::StartAtLogin(true),
    ] {
        assert!(runner.run(&action).unwrap().contains("no system changes"));
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
