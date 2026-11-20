use super::{
    Error, Manifest,
    install::{Bundle, Prepared},
    public_key,
};
use std::{
    fs,
    io::{Read, Write},
    path::PathBuf,
    process::{Command, Stdio},
};

/// The helper acknowledges startup before the parent quits. Its stdin closes on parent exit.
pub(super) fn handoff(prepared: Prepared, target: &Bundle, demo: bool) -> Result<(), Error> {
    let executable = std::env::current_exe()?;
    let mut child = Command::new(executable)
        .arg("--finish-update")
        .arg(target.path())
        .arg(prepared.stage.path())
        .arg(std::process::id().to_string())
        .arg(if demo { "--demo" } else { "--live" })
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()?;
    let mut output = child.stdout.take().ok_or(Error::WorkerStopped)?;
    let mut ready = [0; 1];
    output.read_exact(&mut ready)?;
    if ready != *b"R" {
        return Err(Error::WorkerStopped);
    }
    drop(prepared.stage.keep());
    // The helper waits on the PID too, so closing stdin here cannot race process teardown.
    drop(child.stdin.take());
    Ok(())
}

pub fn finish_if_requested() -> Result<bool, Error> {
    let mut args = std::env::args_os().skip(1);
    if args.next().as_deref() != Some(std::ffi::OsStr::new("--finish-update")) {
        return Ok(false);
    }
    let app = PathBuf::from(args.next().ok_or(Error::Bundle)?);
    let stage = PathBuf::from(args.next().ok_or(Error::Bundle)?);
    let parent: u32 = args
        .next()
        .ok_or(Error::Bundle)?
        .to_str()
        .ok_or(Error::Bundle)?
        .parse()
        .map_err(|_| Error::Bundle)?;
    let demo = args.next().as_deref() == Some(std::ffi::OsStr::new("--demo"));
    let result = (|| -> Result<(), Error> {
        let target = Bundle::discover(&app.join("Contents/MacOS/Reoclo"))?;
        // Reverify the saved archive and extract afresh; nothing mutable from the old process is trusted.
        let manifest = Manifest::parse(&fs::read(stage.join("manifest.json"))?)?;
        let archive = manifest.verify(fs::read(stage.join("archive.tar.gz"))?, &public_key()?)?;
        let prepared = target.prepare(archive)?;
        std::io::stdout().write_all(b"R")?;
        std::io::stdout().flush()?;
        std::io::copy(&mut std::io::stdin(), &mut std::io::sink())?;
        // POSIX shell positional arguments keep paths and the PID out of shell source.
        let waited = Command::new("/bin/sh")
            .args([
                "-c",
                "while kill -0 \"$1\" 2>/dev/null; do sleep 0.1; done",
                "reoclo-update",
            ])
            .arg(parent.to_string())
            .status()?;
        if !waited.success() {
            return Err(Error::WorkerStopped);
        }
        prepared.install(&target, |path| {
            let mut open = Command::new("/usr/bin/open");
            open.args(["-n", "-a"]).arg(path);
            if demo {
                open.args(["--args", "--demo"]);
            }
            if open.status()?.success() {
                Ok(())
            } else {
                Err(std::io::Error::other(
                    "macOS could not open the updated app",
                ))
            }
        })?;
        fs::remove_dir_all(&stage)?;
        Ok(())
    })();
    if let Err(error) = &result {
        // A failed install cannot report to the exited UI. Preserve a local diagnostic.
        fs::write(stage.join("install-error.txt"), error.to_string())?;
    }
    result.map(|()| true)
}
