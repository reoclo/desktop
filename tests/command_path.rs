use reoclo_desktop::system::command_path::{CommandPaths, CommandState, install, remove};
use std::path::{Path, PathBuf};

const MARKER: &str = "# >>> reoclo >>>";

struct Home {
    _dir: tempfile::TempDir,
    paths: CommandPaths,
}

impl Home {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let home = dir.path().to_owned();
        let bundled = app_cli(&home, "Reoclo.app");
        Self {
            _dir: dir,
            paths: CommandPaths { home, bundled },
        }
    }
    fn link(&self) -> PathBuf {
        self.paths.home.join(".local/bin/reoclo")
    }
    fn file(&self, name: &str) -> PathBuf {
        self.paths.home.join(name)
    }
    fn read(&self, name: &str) -> Option<String> {
        std::fs::read_to_string(self.file(name)).ok()
    }
}

/// A bundled CLI that exists, inside an app folder under `home`.
fn app_cli(home: &Path, app: &str) -> PathBuf {
    let cli = home
        .join("Applications")
        .join(app)
        .join("Contents/Resources/reoclo");
    std::fs::create_dir_all(cli.parent().unwrap()).unwrap();
    std::fs::write(&cli, "#!/bin/sh\n").unwrap();
    cli
}

fn nothing() -> Option<PathBuf> {
    None
}

fn no_version(_: &Path) -> Option<String> {
    None
}

fn installed(home: &Home) -> CommandState {
    install(&home.paths, nothing, no_version)
}

#[test]
fn a_fresh_mac_gets_a_link_to_the_bundled_cli_and_one_path_block() {
    let home = Home::new();
    assert_eq!(
        installed(&home),
        CommandState::Installed { link: home.link() }
    );
    assert_eq!(std::fs::read_link(home.link()).unwrap(), home.paths.bundled);
    let zshenv = home.read(".zshenv").unwrap();
    assert_eq!(zshenv.matches(MARKER).count(), 1);
    assert!(zshenv.contains("export PATH=\"$HOME/.local/bin:$PATH\""));
    assert_eq!(home.read(".bash_profile"), None);
}

#[test]
fn running_it_again_changes_nothing_and_writes_no_second_block() {
    let home = Home::new();
    std::fs::write(home.file(".bash_profile"), "export A=1\n").unwrap();
    installed(&home);
    let zshenv = home.read(".zshenv").unwrap();
    let bash = home.read(".bash_profile").unwrap();
    assert_eq!(
        installed(&home),
        CommandState::Installed { link: home.link() }
    );
    assert_eq!(home.read(".zshenv").unwrap(), zshenv);
    assert_eq!(home.read(".bash_profile").unwrap(), bash);
    assert_eq!(bash.matches(MARKER).count(), 1);
    assert!(bash.starts_with("export A=1\n"));
}

#[test]
fn a_homebrew_reoclo_is_left_alone_and_reported() {
    let home = Home::new();
    let state = install(
        &home.paths,
        || Some("/opt/homebrew/bin/reoclo".into()),
        |path| (path == Path::new("/opt/homebrew/bin/reoclo")).then(|| "0.116.1".to_owned()),
    );
    assert_eq!(
        state,
        CommandState::AlreadyPresent {
            path: "/opt/homebrew/bin/reoclo".into(),
            version: Some("0.116.1".into())
        }
    );
    assert!(!home.link().exists());
    assert_eq!(home.read(".zshenv"), None);
}

#[test]
fn an_install_sh_binary_in_the_link_place_is_left_alone_even_off_the_path() {
    let home = Home::new();
    std::fs::create_dir_all(home.link().parent().unwrap()).unwrap();
    std::fs::write(home.link(), "raw binary").unwrap();
    let state = installed(&home);
    assert_eq!(
        state,
        CommandState::AlreadyPresent {
            path: home.link(),
            version: None
        }
    );
    assert_eq!(std::fs::read_to_string(home.link()).unwrap(), "raw binary");
    assert_eq!(home.read(".zshenv"), None);
}

#[test]
fn a_link_into_a_moved_app_is_pointed_at_the_current_one() {
    let home = Home::new();
    let gone = home
        .paths
        .home
        .join("Downloads/Reoclo.app/Contents/Resources/reoclo");
    std::fs::create_dir_all(home.link().parent().unwrap()).unwrap();
    std::os::unix::fs::symlink(&gone, home.link()).unwrap();
    assert_eq!(
        installed(&home),
        CommandState::Installed { link: home.link() }
    );
    assert_eq!(std::fs::read_link(home.link()).unwrap(), home.paths.bundled);
}

#[test]
fn removing_undoes_only_what_the_app_wrote() {
    let home = Home::new();
    std::fs::write(home.file(".zshenv"), "export FOO=1\n").unwrap();
    installed(&home);
    assert_eq!(remove(&home.paths), CommandState::Removed);
    assert!(std::fs::symlink_metadata(home.link()).is_err());
    assert_eq!(home.read(".zshenv").unwrap(), "export FOO=1\n");
}

#[test]
fn a_shell_file_without_a_final_newline_keeps_its_last_line_apart_from_the_block() {
    let home = Home::new();
    std::fs::write(home.file(".zshenv"), "export FOO=1").unwrap();
    installed(&home);
    assert!(
        home.read(".zshenv")
            .unwrap()
            .starts_with("export FOO=1\n# >>>")
    );
    remove(&home.paths);
    assert_eq!(home.read(".zshenv").unwrap().trim_end(), "export FOO=1");
}

#[test]
fn removing_a_file_that_held_only_the_block_deletes_it() {
    let home = Home::new();
    installed(&home);
    assert!(home.read(".zshenv").unwrap().contains(MARKER));
    remove(&home.paths);
    assert_eq!(home.read(".zshenv"), None);
}

#[test]
fn removing_leaves_a_foreign_reoclo_and_a_foreign_block_free_file_alone() {
    let home = Home::new();
    std::fs::create_dir_all(home.link().parent().unwrap()).unwrap();
    std::fs::write(home.link(), "raw binary").unwrap();
    std::fs::write(home.file(".zshenv"), "export FOO=1\n").unwrap();
    assert_eq!(remove(&home.paths), CommandState::Removed);
    assert_eq!(std::fs::read_to_string(home.link()).unwrap(), "raw binary");
    assert_eq!(home.read(".zshenv").unwrap(), "export FOO=1\n");
}
