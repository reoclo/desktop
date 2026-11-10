use reoclo_desktop::cli::{
    Cli, Locator, MIN_VERSION, at_least, older, service_program, upgrade_command,
};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

const BUNDLED: &str = "/Applications/Reoclo.app/Contents/Resources/reoclo";
const BREW: &str = "/opt/homebrew/bin/reoclo";

fn locator() -> Locator {
    Locator {
        home: "/Users/me".into(),
        bundled: Some(BUNDLED.into()),
        ..Default::default()
    }
}

/// `reoclo --version` for the paths that exist in a test, and nothing for the rest.
fn versions(known: &[(&str, &str)]) -> impl Fn(&Path) -> Option<String> {
    let known: HashMap<PathBuf, String> = known
        .iter()
        .map(|(path, version)| (PathBuf::from(path), (*version).to_owned()))
        .collect();
    move |path| known.get(path).cloned()
}

fn ready(path: &str, version: &str, bundled: bool) -> Cli {
    Cli::Ready {
        path: path.into(),
        version: version.into(),
        bundled,
    }
}

#[test]
fn candidates_start_with_the_override_and_the_daemon_then_the_usual_installs() {
    let mut locator = locator();
    locator.override_path = Some("/tmp/dev/reoclo".into());
    assert_eq!(
        locator.candidates(Some(BREW)),
        [
            "/tmp/dev/reoclo",
            BREW,
            "/usr/local/bin/reoclo",
            "/Users/me/.local/bin/reoclo"
        ]
        .map(PathBuf::from)
    );
}

#[test]
fn the_launch_agent_program_comes_before_the_usual_installs() {
    let dir = tempfile::tempdir().unwrap();
    let plist = dir.path().join("com.reoclo.tunneld.plist");
    std::fs::write(
        &plist,
        "<plist version=\"1.0\">\n<dict>\n  <key>Label</key>\n  <string>com.reoclo.tunneld</string>\n  <key>ProgramArguments</key>\n  <array>\n    <string>/Users/me/.local/bin/reoclo</string>\n    <string>tunnel</string>\n  </array>\n</dict>\n</plist>\n",
    )
    .unwrap();
    let mut locator = locator();
    locator.service_plist = Some(plist);
    assert_eq!(
        locator.candidates(None)[0],
        PathBuf::from("/Users/me/.local/bin/reoclo")
    );
}

#[test]
fn the_service_program_is_unescaped() {
    assert_eq!(
        service_program(
            "<key>ProgramArguments</key><array><string>/tmp/A&amp;B/reoclo</string></array>"
        )
        .as_deref(),
        Some("/tmp/A&B/reoclo")
    );
    assert_eq!(service_program("<key>Label</key>"), None);
}

#[test]
fn the_installed_cli_wins_over_the_bundled_copy() {
    let found = locator().locate(None, versions(&[(BREW, "0.104.0"), (BUNDLED, "0.103.1")]));
    assert_eq!(found, ready(BREW, "0.104.0", false));
}

#[test]
fn the_binary_the_daemon_reports_comes_first() {
    let found = locator().locate(
        Some("/usr/local/bin/reoclo"),
        versions(&[("/usr/local/bin/reoclo", "0.104.0"), (BREW, "0.105.0")]),
    );
    assert_eq!(found, ready("/usr/local/bin/reoclo", "0.104.0", false));
}

#[test]
fn a_daemon_on_the_bundled_copy_moves_to_the_installed_cli() {
    let found = locator().locate(
        Some(BUNDLED),
        versions(&[(BUNDLED, "0.103.1"), (BREW, "0.104.0")]),
    );
    assert_eq!(found, ready(BREW, "0.104.0", false));
}

#[test]
fn a_new_enough_install_later_in_the_list_beats_an_old_one() {
    let found = locator().locate(
        Some("/usr/local/bin/reoclo"),
        versions(&[("/usr/local/bin/reoclo", "0.98.0"), (BREW, "0.104.0")]),
    );
    assert_eq!(found, ready(BREW, "0.104.0", false));
}

#[test]
fn an_old_install_asks_for_an_upgrade_instead_of_using_the_bundled_copy() {
    let found = locator().locate(None, versions(&[(BREW, "0.98.0"), (BUNDLED, "0.104.0")]));
    assert_eq!(
        found,
        Cli::TooOld {
            path: BREW.into(),
            version: "0.98.0".into()
        }
    );
}

#[test]
fn without_an_install_the_bundled_copy_runs() {
    let found = locator().locate(None, versions(&[(BUNDLED, "0.103.1")]));
    assert_eq!(found, ready(BUNDLED, "0.103.1", true));
    assert_eq!(locator().locate(None, versions(&[])), Cli::Missing);
}

#[test]
fn versions_compare_by_number() {
    assert!(at_least("v0.104.0", MIN_VERSION));
    assert!(!at_least("0.98.0", MIN_VERSION));
    assert!(older("0.103.2", "0.104.0"));
    assert!(older("0.9.0", "0.10.0"));
    assert!(!older("0.104.0", "0.104.0"));
    assert!(!older("dev", "0.104.0"));
}

#[test]
fn a_homebrew_install_upgrades_with_brew() {
    assert_eq!(
        upgrade_command(Path::new("/opt/homebrew/Cellar/reoclo/0.104.0/bin/reoclo")),
        "brew upgrade reoclo"
    );
    assert_eq!(
        upgrade_command(Path::new("/nonexistent/bin/reoclo")),
        "reoclo upgrade"
    );
}
