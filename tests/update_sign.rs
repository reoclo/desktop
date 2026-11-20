use base64::{Engine, engine::general_purpose::STANDARD};
use ed25519_dalek::SigningKey;
use reoclo_desktop::update::Manifest;
use std::{fs, io::Read, process::Command};

#[test]
fn signer_writes_a_manifest_that_verifies_with_the_generated_public_key() {
    let dir = tempfile::tempdir().unwrap();
    let archive = dir.path().join("Reoclo.app.tar.gz");
    let notes = dir.path().join("notes.txt");
    let output = dir.path().join("latest.json");
    fs::write(&archive, b"release archive").unwrap();
    fs::write(&notes, "Test release.").unwrap();
    let mut seed = [0; 32];
    fs::File::open("/dev/urandom")
        .unwrap()
        .read_exact(&mut seed)
        .unwrap();
    let key = SigningKey::from_bytes(&seed);
    let result = Command::new(env!("CARGO_BIN_EXE_update-sign"))
        .arg(&archive)
        .args(["0.6.0", "2026-11-20T00:00:00Z"])
        .arg(notes)
        .arg("https://example.com/Reoclo.app.tar.gz")
        .arg(&output)
        .arg("--test-key")
        .env("UPDATE_SIGNING_KEY", STANDARD.encode(seed))
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let manifest = Manifest::parse(&fs::read(output).unwrap()).unwrap();
    assert!(
        manifest
            .verify(fs::read(archive).unwrap(), &key.verifying_key())
            .is_ok()
    );
    assert!(!String::from_utf8_lossy(&result.stdout).contains(&STANDARD.encode(seed)));
}
