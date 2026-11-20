//! Local release exercise with a generated signing key. Never launches a live app.
use base64::{Engine, engine::general_purpose::STANDARD};
use ed25519_dalek::SigningKey;
use reoclo_desktop::update::{Client, install::Bundle};
use std::{
    fs,
    io::{BufRead, BufReader, Read, Write},
    net::TcpListener,
    path::PathBuf,
    process::Command,
    time::Duration,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    if args.next().as_deref() == Some("--install") {
        let root = PathBuf::from(args.next().ok_or("Missing test bundle")?);
        let public: [u8; 32] = STANDARD
            .decode(args.next().ok_or("Missing test public key")?)?
            .try_into()
            .map_err(|_| "Invalid test public key")?;
        let key = ed25519_dalek::VerifyingKey::from_bytes(&public)?;
        let client = Client::new(std::env::var("REOCLO_UPDATE_URL")?, key)?;
        let release = client.check("0.4.0")?.ok_or("No newer release")?;
        let expected = release.version.to_string();
        let bundle = Bundle::discover(&root.join("Reoclo.app/Contents/MacOS/Reoclo"))?;
        let prepared = bundle.prepare(client.download(release)?)?;
        prepared.install(&bundle, |app| {
            // A test installation validates the real release bundle without opening a live app.
            let status = Command::new("/usr/bin/codesign")
                .args(["--verify", "--deep", "--strict"])
                .arg(app)
                .status()?;
            if status.success() {
                Ok(())
            } else {
                Err(std::io::Error::other("Installed bundle signature failed"))
            }
        })?;
        let installed = plist::Value::from_file(bundle.path().join("Contents/Info.plist"))?;
        let version = installed
            .as_dictionary()
            .and_then(|d| d.get("CFBundleShortVersionString"))
            .and_then(plist::Value::as_string)
            .ok_or("Missing installed version")?;
        if version != expected || fs::read_dir(&root)?.count() != 1 {
            return Err("Bundle swap or old-copy cleanup failed".into());
        }
        println!(
            "UPDATE INSTALL PASS: 0.4.0 -> {version}; archive verified, bundle swapped, codesign valid, old copy removed"
        );
        return Ok(());
    }
    fs::create_dir_all(".cache/tmp")?;
    fs::create_dir_all("evidence")?;
    let root = tempfile::tempdir_in(".cache/tmp")?;
    let executable = root.path().join("Reoclo.app/Contents/MacOS/Reoclo");
    fs::create_dir_all(executable.parent().ok_or("Missing parent")?)?;
    fs::write(&executable, "old app")?;
    let mut seed = [0; 32];
    fs::File::open("/dev/urandom")?.read_exact(&mut seed)?;
    let key = SigningKey::from_bytes(&seed);
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let address = listener.local_addr()?;
    let base = format!("http://{address}");
    let packaged = Command::new("bash")
        .args([
            "scripts/package-release.sh",
            env!("CARGO_PKG_VERSION"),
            "local-test",
            "target/debug/update-sign",
            "--test-key",
        ])
        .env("UPDATE_SIGNING_KEY", STANDARD.encode(seed))
        .env("REOCLO_RELEASE_BASE_URL", &base)
        .status()?;
    if !packaged.success() {
        return Err("Release packaging failed".into());
    }
    let server = std::thread::spawn(move || -> std::io::Result<()> {
        for file in [
            "latest.json".to_owned(),
            format!("Reoclo-{}.app.tar.gz", env!("CARGO_PKG_VERSION")),
        ] {
            let bytes = fs::read(PathBuf::from("dist").join(&file))?;
            let (mut stream, _) = listener.accept()?;
            stream.set_read_timeout(Some(Duration::from_secs(10)))?;
            stream.set_write_timeout(Some(Duration::from_secs(60)))?;
            let mut request = String::new();
            BufReader::new(&stream).read_line(&mut request)?;
            if !request.starts_with(&format!("GET /{file} ")) {
                return Err(std::io::Error::other("Unexpected update request"));
            }
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                bytes.len()
            )?;
            stream.write_all(&bytes)?;
        }
        Ok(())
    });
    let result = Command::new(std::env::current_exe()?)
        .arg("--install")
        .arg(root.path())
        .arg(STANDARD.encode(key.verifying_key().to_bytes()))
        .env("REOCLO_UPDATE_URL", format!("{base}/latest.json"))
        .status()?;
    if !result.success() {
        return Err("Local update installation failed".into());
    }
    server.join().map_err(|_| "Local server panicked")??;
    Ok(())
}
