use base64::{Engine, engine::general_purpose::STANDARD};
use ed25519_dalek::{Signer, SigningKey};
use reoclo_desktop::update::{Manifest, public_key};
use sha2::{Digest, Sha256};
use std::{env, fs};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().skip(1).collect();
    let test_key = args.get(6).is_some_and(|arg| arg == "--test-key");
    if args.len() != 6 && !(args.len() == 7 && test_key) {
        return Err(
            "Usage: update-sign ARCHIVE VERSION PUB_DATE NOTES_FILE URL OUTPUT_JSON [--test-key]"
                .into(),
        );
    }
    let seed: [u8; 32] = STANDARD
        .decode(
            env::var("UPDATE_SIGNING_KEY")
                .map_err(|_| "UPDATE_SIGNING_KEY is required")?
                .trim(),
        )
        .map_err(|_| "UPDATE_SIGNING_KEY must be base64")?
        .try_into()
        .map_err(|_| "UPDATE_SIGNING_KEY must contain a 32-byte seed")?;
    let key = SigningKey::from_bytes(&seed);
    if !test_key && key.verifying_key() != public_key()? {
        return Err("The signing seed does not match assets/update-public-key.txt".into());
    }
    let bytes = fs::read(&args[0])?;
    let manifest = Manifest {
        version: args[1].parse()?,
        pub_date: args[2].clone(),
        notes: fs::read_to_string(&args[3])?,
        url: args[4].clone(),
        sha256: format!("{:x}", Sha256::digest(&bytes)),
        signature: STANDARD.encode(key.sign(&bytes).to_bytes()),
    };
    let json = serde_json::to_vec_pretty(&manifest)?;
    Manifest::parse(&json)?.verify(bytes, &key.verifying_key())?;
    fs::write(&args[5], json)?;
    println!("Signed update manifest: {}", args[5]);
    Ok(())
}
