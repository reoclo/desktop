use super::Error;
use base64::{Engine, engine::general_purpose::STANDARD};
use ed25519_dalek::{Signature, VerifyingKey};
use semver::Version;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Manifest {
    pub version: Version,
    pub pub_date: String,
    pub notes: String,
    pub url: String,
    pub sha256: String,
    pub signature: String,
}

impl Manifest {
    pub fn parse(bytes: &[u8]) -> Result<Self, Error> {
        let manifest: Self = serde_json::from_slice(bytes)?;
        time::OffsetDateTime::parse(
            &manifest.pub_date,
            &time::format_description::well_known::Rfc3339,
        )
        .map_err(|_| Error::Manifest("publication date"))?;
        validate_url(&manifest.url)?;
        if manifest.sha256.len() != 64 || !manifest.sha256.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(Error::Manifest("SHA-256 checksum"));
        }
        if STANDARD
            .decode(&manifest.signature)
            .map_or(true, |s| s.len() != 64)
        {
            return Err(Error::Manifest("signature"));
        }
        Ok(manifest)
    }

    pub fn is_newer_than(&self, current: &str) -> Result<bool, Error> {
        Ok(self
            .version
            .cmp_precedence(&Version::parse(current)?)
            .is_gt())
    }

    pub fn verify(self, bytes: Vec<u8>, key: &VerifyingKey) -> Result<VerifiedArchive, Error> {
        if format!("{:x}", Sha256::digest(&bytes)) != self.sha256.to_ascii_lowercase() {
            return Err(Error::Checksum);
        }
        let signature = STANDARD
            .decode(&self.signature)
            .map_err(|_| Error::Signature)?;
        let signature = Signature::from_slice(&signature).map_err(|_| Error::Signature)?;
        key.verify_strict(&bytes, &signature)
            .map_err(|_| Error::Signature)?;
        Ok(VerifiedArchive {
            manifest: self,
            bytes,
        })
    }
}

/// Only signature verification can construct an archive eligible for extraction.
pub struct VerifiedArchive {
    pub manifest: Manifest,
    pub(super) bytes: Vec<u8>,
}

pub fn public_key() -> Result<VerifyingKey, Error> {
    let raw: [u8; 32] = STANDARD
        .decode(include_str!("../../assets/update-public-key.txt").trim())
        .map_err(|_| Error::PublicKey)?
        .try_into()
        .map_err(|_| Error::PublicKey)?;
    VerifyingKey::from_bytes(&raw).map_err(|_| Error::PublicKey)
}

pub(super) fn validate_url(url: &str) -> Result<(), Error> {
    let uri: ureq::http::Uri = url.parse().map_err(|_| Error::Manifest("download URL"))?;
    let loopback = matches!(uri.host(), Some("localhost" | "127.0.0.1" | "[::1]"));
    match (uri.scheme_str(), uri.host()) {
        (Some("https"), Some(_)) => Ok(()),
        (Some("http"), Some(_)) if loopback => Ok(()),
        _ => Err(Error::Manifest("download URL")),
    }
}
