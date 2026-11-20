use super::{Error, Manifest, VerifiedArchive, manifest::validate_url};
use ed25519_dalek::VerifyingKey;
use std::time::Duration;

pub struct Client {
    agent: ureq::Agent,
    endpoint: String,
    key: VerifyingKey,
}

impl Client {
    pub fn new(endpoint: String, key: VerifyingKey) -> Result<Self, Error> {
        validate_url(&endpoint)?;
        let agent = ureq::Agent::config_builder()
            .timeout_global(Some(Duration::from_secs(300)))
            .timeout_connect(Some(Duration::from_secs(15)))
            .max_redirects(5)
            .build()
            .into();
        Ok(Self {
            agent,
            endpoint,
            key,
        })
    }

    fn get(&self, url: &str, limit: u64) -> Result<Vec<u8>, Error> {
        validate_url(url)?;
        Ok(self
            .agent
            .get(url)
            .config()
            .https_only(url.starts_with("https://"))
            .build()
            .call()?
            .body_mut()
            .with_config()
            .limit(limit)
            .read_to_vec()?)
    }

    pub fn check(&self, current: &str) -> Result<Option<Manifest>, Error> {
        let manifest = Manifest::parse(&self.get(&self.endpoint, 1024 * 1024)?)?;
        Ok(manifest.is_newer_than(current)?.then_some(manifest))
    }

    pub fn download(&self, manifest: Manifest) -> Result<VerifiedArchive, Error> {
        let bytes = self.get(&manifest.url, 512 * 1024 * 1024)?;
        manifest.verify(bytes, &self.key)
    }
}
