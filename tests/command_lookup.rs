use reoclo_desktop::system::command_path::{find, shell_lookup_with};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

fn shell(dir: &Path, body: &str) -> PathBuf {
    let path = dir.join("fake-zsh");
    std::fs::write(&path, format!("#!/bin/sh\n{body}\n")).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    path
}

#[test]
fn a_shell_that_prints_the_command_is_believed() {
    let dir = tempfile::tempdir().unwrap();
    let zsh = shell(
        dir.path(),
        "echo 'welcome back'; echo /opt/homebrew/bin/reoclo",
    );
    assert_eq!(
        shell_lookup_with(&zsh, Duration::from_secs(10)),
        Some(PathBuf::from("/opt/homebrew/bin/reoclo"))
    );
}

#[test]
fn a_shell_that_hangs_on_a_heavy_zshrc_gives_up_at_the_limit() {
    let dir = tempfile::tempdir().unwrap();
    let zsh = shell(dir.path(), "exec sleep 60");
    let started = Instant::now();
    assert_eq!(shell_lookup_with(&zsh, Duration::from_millis(300)), None);
    assert!(started.elapsed() < Duration::from_secs(20));
}

#[test]
fn a_shell_that_cannot_start_finds_nothing() {
    assert_eq!(
        shell_lookup_with(Path::new("/nonexistent/zsh"), Duration::from_secs(1)),
        None
    );
}

#[test]
fn the_known_install_paths_answer_when_the_shell_does_not() {
    let dir = tempfile::tempdir().unwrap();
    let installed = dir.path().join("bin/reoclo");
    std::fs::create_dir_all(installed.parent().unwrap()).unwrap();
    std::fs::write(&installed, "#!/bin/sh\n").unwrap();
    let known = vec![dir.path().join("missing/reoclo"), installed.clone()];
    assert_eq!(find(|| None, known.clone()), Some(installed));
    assert_eq!(
        find(|| Some(PathBuf::from("/shell/reoclo")), known),
        Some(PathBuf::from("/shell/reoclo"))
    );
    assert_eq!(find(|| None, vec![dir.path().join("missing/reoclo")]), None);
}
