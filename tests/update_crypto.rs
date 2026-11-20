use base64::{Engine, engine::general_purpose::STANDARD};
use ed25519_dalek::{Signer, SigningKey};
use reoclo_desktop::update::{Error, Manifest};
use sha2::{Digest, Sha256};
use std::io::Read;

#[test]
fn verification_requires_both_the_checksum_and_the_trusted_signature() {
    let mut seed = [0; 32];
    std::fs::File::open("/dev/urandom")
        .unwrap()
        .read_exact(&mut seed)
        .unwrap();
    let key = SigningKey::from_bytes(&seed);
    let bytes = b"archive bytes".to_vec();
    let manifest = Manifest {
        version: "0.6.0".parse().unwrap(),
        pub_date: "2026-11-20T00:00:00Z".into(),
        notes: String::new(),
        url: "https://example.com/app.tar.gz".into(),
        sha256: format!("{:x}", Sha256::digest(&bytes)),
        signature: STANDARD.encode(key.sign(&bytes).to_bytes()),
    };
    assert!(
        manifest
            .clone()
            .verify(bytes.clone(), &key.verifying_key())
            .is_ok()
    );
    assert!(matches!(
        manifest
            .clone()
            .verify(b"tampered".to_vec(), &key.verifying_key()),
        Err(Error::Checksum)
    ));
    let mut tampered = manifest.clone();
    tampered.sha256 = format!("{:x}", Sha256::digest(b"tampered"));
    assert!(matches!(
        tampered.verify(b"tampered".to_vec(), &key.verifying_key()),
        Err(Error::Signature)
    ));
    seed[0] ^= 1;
    assert!(matches!(
        manifest.verify(bytes, &SigningKey::from_bytes(&seed).verifying_key()),
        Err(Error::Signature)
    ));
}
