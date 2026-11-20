use base64::{Engine, engine::general_purpose::STANDARD};
use ed25519_dalek::{Signer, SigningKey};
use reoclo_desktop::update::{Manifest, VerifiedArchive};
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
};

pub fn bundle(root: &Path, version: &str) -> PathBuf {
    use std::os::unix::fs::{PermissionsExt, symlink};
    let app = root.join("Reoclo.app/Contents");
    fs::create_dir_all(app.join("MacOS")).unwrap();
    fs::create_dir_all(app.join("Resources")).unwrap();
    let executable = app.join("MacOS/Reoclo");
    fs::write(&executable, version).unwrap();
    fs::set_permissions(&executable, fs::Permissions::from_mode(0o755)).unwrap();
    symlink("../MacOS/Reoclo", app.join("Resources/current")).unwrap();
    fs::write(app.join("Info.plist"), format!(
        "<plist version=\"1.0\"><dict><key>CFBundleShortVersionString</key><string>{version}</string><key>CFBundleIdentifier</key><string>com.reoclo.desktop</string><key>CFBundleExecutable</key><string>Reoclo</string></dict></plist>"
    )).unwrap();
    executable
}

pub fn signed(bytes: Vec<u8>, version: &str) -> (Manifest, Vec<u8>, SigningKey) {
    let mut seed = [0; 32];
    use std::io::Read;
    fs::File::open("/dev/urandom")
        .unwrap()
        .read_exact(&mut seed)
        .unwrap();
    let key = SigningKey::from_bytes(&seed);
    let manifest = Manifest {
        version: version.parse().unwrap(),
        pub_date: "2026-11-20T00:00:00Z".into(),
        notes: "Desktop improvements.".into(),
        url: "https://example.com/app.tar.gz".into(),
        sha256: format!("{:x}", Sha256::digest(&bytes)),
        signature: STANDARD.encode(key.sign(&bytes).to_bytes()),
    };
    (manifest, bytes, key)
}

pub fn release(version: &str) -> VerifiedArchive {
    let root = tempfile::tempdir().unwrap();
    bundle(root.path(), version);
    let encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
    let mut archive = tar::Builder::new(encoder);
    archive.follow_symlinks(false);
    archive
        .append_dir_all("Reoclo.app", root.path().join("Reoclo.app"))
        .unwrap();
    let bytes = archive.into_inner().unwrap().finish().unwrap();
    let (manifest, bytes, key) = signed(bytes, version);
    manifest.verify(bytes, &key.verifying_key()).unwrap()
}
