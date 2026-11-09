//! An unprivileged development peer for the desktop client.
use reoclo_desktop::fake::FakeDaemon;
use std::io::{BufRead, Write};
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args()
        .nth(1)
        .ok_or("Pass an explicit socket path inside a development directory")?;
    let fake = FakeDaemon::start(PathBuf::from(path))?;
    println!("READY {}", fake.path().display());
    std::io::stdout().flush()?;
    for line in std::io::stdin().lock().lines() {
        if line? == "quit" {
            break;
        }
    }
    drop(fake);
    println!("STOPPED");
    Ok(())
}
