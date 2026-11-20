//! Puts the bundled CLI where an agent's shell finds it, without administrator rights.

use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::Duration;

/// How long the login shell has to answer. A heavy .zshrc can take longer, and the app must not wait for it.
const SHELL_SECONDS: u64 = 5;

const START: &str = "# >>> reoclo >>>";
const END: &str = "# <<< reoclo <<<";
const BLOCK_BODY: &str = "case \":$PATH:\" in *\":$HOME/.local/bin:\"*) ;; *) export PATH=\"$HOME/.local/bin:$PATH\" ;; esac";

/// Where the app writes: the home directory it links under, and the CLI it links to.
#[derive(Clone, Debug)]
pub struct CommandPaths {
    pub home: PathBuf,
    /// `Reoclo.app/Contents/Resources/reoclo`.
    pub bundled: PathBuf,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CommandState {
    /// `reoclo` on an agent's PATH is the app's copy, through this link.
    Installed {
        link: PathBuf,
    },
    /// Another install already answers to `reoclo`, and the app left it alone.
    AlreadyPresent {
        path: PathBuf,
        version: Option<String>,
    },
    /// What the app wrote is gone. Nothing else was touched.
    Removed,
    Failed(String),
}

impl CommandPaths {
    fn link(&self) -> PathBuf {
        self.home.join(".local/bin/reoclo")
    }

    /// The files an agent's shell reads for PATH: always `.zshenv`, and `.bash_profile` once it exists.
    fn shell_files(&self) -> Vec<PathBuf> {
        let zsh = self.home.join(".zshenv");
        let bash = self.home.join(".bash_profile");
        if bash.exists() {
            vec![zsh, bash]
        } else {
            vec![zsh]
        }
    }
}

/// What `reoclo` resolves to in the login shell an agent starts, as an absolute path.
pub fn shell_lookup() -> Option<PathBuf> {
    shell_lookup_with(Path::new("/bin/zsh"), Duration::from_secs(SHELL_SECONDS))
}

/// Asks `shell` for `reoclo`, and gives up when it has not answered within `limit`.
pub fn shell_lookup_with(shell: &Path, limit: Duration) -> Option<PathBuf> {
    let mut child = Command::new(shell)
        .args(["-lic", "command -v reoclo"])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    let mut stdout = child.stdout.take()?;
    let (answer, printed) = mpsc::channel();
    std::thread::spawn(move || {
        let mut text = String::new();
        let read = stdout.read_to_string(&mut text);
        // The receiver is gone when the shell ran past the limit.
        let _ = answer.send(read.map(|_| text));
    });
    let text = match printed.recv_timeout(limit) {
        Ok(Ok(text)) => text,
        _ => {
            // Killing a process that already exited is not an error worth reporting.
            let _ = child.kill();
            let _ = child.wait();
            return None;
        }
    };
    let _ = child.wait();
    absolute_line(&text)
}

/// The first of `known` that exists when the shell cannot say, such as the usual Homebrew folders.
pub fn find(shell: impl FnOnce() -> Option<PathBuf>, known: Vec<PathBuf>) -> Option<PathBuf> {
    shell().or_else(|| known.into_iter().find(|path| path.is_file()))
}

/// Where `reoclo` is on this Mac: what the login shell finds, or else a known install folder.
pub fn lookup(home: &Path) -> Option<PathBuf> {
    let known = crate::cli::Locator {
        home: home.to_owned(),
        ..Default::default()
    };
    find(shell_lookup, known.candidates(None))
}

fn absolute_line(text: &str) -> Option<PathBuf> {
    text.lines()
        .rev()
        .map(str::trim)
        .find(|line| line.starts_with('/'))
        .map(PathBuf::from)
}

/// A link the app wrote points into some app bundle's copy of the CLI.
fn is_ours(target: &Path) -> bool {
    target.ends_with("Contents/Resources/reoclo")
        && target
            .ancestors()
            .any(|dir| dir.extension().is_some_and(|ext| ext == "app"))
}

fn own_link_target(link: &Path) -> Option<PathBuf> {
    std::fs::read_link(link)
        .ok()
        .filter(|target| is_ours(target))
}

fn foreign(path: PathBuf, version_of: &impl Fn(&Path) -> Option<String>) -> CommandState {
    let version = version_of(&path);
    CommandState::AlreadyPresent { path, version }
}

/// Links the bundled CLI and adds the PATH block, unless another `reoclo` is already in charge.
pub fn install(
    paths: &CommandPaths,
    lookup: impl Fn() -> Option<PathBuf>,
    version_of: impl Fn(&Path) -> Option<String>,
) -> CommandState {
    let link = paths.link();
    if let Some(found) = lookup().filter(|found| *found != link) {
        return foreign(found, &version_of);
    }
    match std::fs::symlink_metadata(&link) {
        Ok(_) if own_link_target(&link).is_none() => return foreign(link, &version_of),
        Ok(_) if own_link_target(&link).as_deref() == Some(paths.bundled.as_path()) => {}
        Ok(_) => {
            if let Err(error) = std::fs::remove_file(&link).and_then(|()| point(paths, &link)) {
                return CommandState::Failed(error.to_string());
            }
        }
        Err(_) => {
            if let Err(error) = point(paths, &link) {
                return CommandState::Failed(error.to_string());
            }
        }
    }
    match paths
        .shell_files()
        .iter()
        .try_for_each(|file| edit(file, insert_block))
    {
        Ok(()) => CommandState::Installed { link },
        Err(error) => CommandState::Failed(error.to_string()),
    }
}

fn point(paths: &CommandPaths, link: &Path) -> std::io::Result<()> {
    if let Some(parent) = link.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::os::unix::fs::symlink(&paths.bundled, link)
}

/// Removes the link and the PATH block the app wrote, and nothing else.
pub fn remove(paths: &CommandPaths) -> CommandState {
    let link = paths.link();
    let unlinked = if own_link_target(&link).is_some() {
        std::fs::remove_file(&link)
    } else {
        Ok(())
    };
    let result = unlinked.and_then(|()| {
        paths
            .shell_files()
            .iter()
            .try_for_each(|file| edit(file, remove_block))
    });
    match result {
        Ok(()) => CommandState::Removed,
        Err(error) => CommandState::Failed(error.to_string()),
    }
}

/// Rewrites a shell file through `change`, and deletes it when nothing is left. A missing file reads as empty.
fn edit(file: &Path, change: fn(&str) -> String) -> std::io::Result<()> {
    let before = match std::fs::read_to_string(file) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(error) => return Err(error),
    };
    let after = change(&before);
    if after == before {
        Ok(())
    } else if after.trim().is_empty() {
        std::fs::remove_file(file)
    } else {
        std::fs::write(file, after)
    }
}

fn insert_block(text: &str) -> String {
    if text.contains(START) {
        return text.to_owned();
    }
    let gap = if text.is_empty() || text.ends_with('\n') {
        ""
    } else {
        "\n"
    };
    format!("{text}{gap}{START}\n{BLOCK_BODY}\n{END}\n")
}

fn remove_block(text: &str) -> String {
    let Some(start) = text.find(START) else {
        return text.to_owned();
    };
    let Some(end) = text[start..].find(END).map(|at| start + at + END.len()) else {
        return text.to_owned();
    };
    let tail = text[end..].strip_prefix('\n').unwrap_or(&text[end..]);
    format!("{}{tail}", &text[..start])
}
