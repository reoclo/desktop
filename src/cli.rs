//! Which reoclo CLI the app runs for the actions that need one. The CLI already
//! installed on the Mac owns the daemon, so `brew upgrade` reaches it. The copy
//! bundled with the app is only for a Mac without one.

use std::path::{Path, PathBuf};
use std::process::Command;

/// The oldest CLI with every command and flag the app passes to it.
pub const MIN_VERSION: &str = "0.103.1";

/// Where the usual installs put the CLI: Homebrew on Apple silicon, then Homebrew on Intel and install.sh.
const INSTALL_DIRS: [&str; 2] = ["/opt/homebrew/bin", "/usr/local/bin"];

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Cli {
    Ready {
        path: PathBuf,
        version: String,
        bundled: bool,
    },
    /// An installed CLI older than the app needs. The app asks for an upgrade rather
    /// than move the daemon to its bundled copy.
    TooOld { path: PathBuf, version: String },
    /// No installed CLI and no bundled copy.
    Missing,
}

/// What the search reads besides the daemon's own answer.
#[derive(Clone, Debug, Default)]
pub struct Locator {
    /// `REOCLO_CLI`, for development.
    pub override_path: Option<PathBuf>,
    /// The daemon's launch agent. Its program is the CLI that runs at login.
    pub service_plist: Option<PathBuf>,
    pub home: PathBuf,
    pub bundled: Option<PathBuf>,
}

impl Locator {
    /// Installed CLIs to try, in order: the override, the binary the daemon reports, the
    /// launch agent's program, then the usual install locations.
    pub fn candidates(&self, daemon: Option<&str>) -> Vec<PathBuf> {
        let service = self
            .service_plist
            .as_deref()
            .and_then(|plist| std::fs::read_to_string(plist).ok())
            .and_then(|text| service_program(&text));
        let listed = self
            .override_path
            .clone()
            .into_iter()
            .chain(daemon.map(PathBuf::from))
            .chain(service.map(PathBuf::from))
            .chain(INSTALL_DIRS.iter().map(|dir| Path::new(dir).join("reoclo")))
            .chain(std::iter::once(self.home.join(".local/bin/reoclo")));
        let mut out = Vec::new();
        for path in listed {
            if !out.contains(&path) {
                out.push(path);
            }
        }
        out
    }

    /// The first installed CLI that is new enough. `version_of` answers `reoclo --version`,
    /// or None when the path does not run.
    pub fn locate(
        &self,
        daemon: Option<&str>,
        version_of: impl Fn(&Path) -> Option<String>,
    ) -> Cli {
        let bundled = self.bundled.as_deref().map(canonical);
        let mut too_old = None;
        for path in self.candidates(daemon) {
            // A daemon started by an older app runs the bundled copy; that is not an install.
            if bundled.as_deref() == Some(canonical(&path).as_path()) {
                continue;
            }
            let Some(version) = version_of(&path) else {
                continue;
            };
            if at_least(&version, MIN_VERSION) {
                return Cli::Ready {
                    path,
                    version,
                    bundled: false,
                };
            }
            too_old.get_or_insert(Cli::TooOld { path, version });
        }
        if let Some(cli) = too_old {
            return cli;
        }
        match self
            .bundled
            .as_ref()
            .and_then(|path| version_of(path).map(|version| (path.clone(), version)))
        {
            Some((path, version)) => Cli::Ready {
                path,
                version,
                bundled: true,
            },
            None => Cli::Missing,
        }
    }
}

fn canonical(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| path.to_owned())
}

/// Runs `<path> --version`. None when the file is missing, does not run, or prints no version.
pub fn version_of(path: &Path) -> Option<String> {
    if !path.is_file() {
        return None;
    }
    let output = Command::new(path).arg("--version").output().ok()?;
    if !output.status.success() {
        return None;
    }
    let printed = String::from_utf8_lossy(&output.stdout);
    let version = printed.split_whitespace().next()?.to_owned();
    parse(&version).map(|_| version)
}

fn parse(version: &str) -> Option<(u64, u64, u64)> {
    let core = version.trim().trim_start_matches('v');
    let core = core.split(['-', '+']).next()?;
    let mut parts = core.split('.').map(|part| part.parse::<u64>().ok());
    Some((parts.next()??, parts.next()??, parts.next()??))
}

/// True when `version` is `min` or newer. False when either does not parse.
pub fn at_least(version: &str, min: &str) -> bool {
    matches!((parse(version), parse(min)), (Some(v), Some(m)) if v >= m)
}

/// True when `a` is older than `b`. False when either does not parse.
pub fn older(a: &str, b: &str) -> bool {
    matches!((parse(a), parse(b)), (Some(a), Some(b)) if a < b)
}

/// The first program argument in a launchd plist: the CLI the daemon service runs.
pub fn service_program(plist: &str) -> Option<String> {
    let after = &plist[plist.find("<key>ProgramArguments</key>")?..];
    let start = after.find("<string>")? + "<string>".len();
    let end = start + after[start..].find("</string>")?;
    Some(
        after[start..end]
            .replace("&lt;", "<")
            .replace("&gt;", ">")
            .replace("&quot;", "\"")
            .replace("&amp;", "&"),
    )
}

pub fn upgrade_command(path: &Path) -> &'static str {
    if canonical(path).to_string_lossy().contains("/Cellar/") {
        "brew upgrade reoclo"
    } else {
        "reoclo upgrade"
    }
}
