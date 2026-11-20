const RESERVED: [&str; 6] = [
    "test",
    "localhost",
    "internal",
    "home.arpa",
    "example",
    "invalid",
];

#[derive(Debug, thiserror::Error)]
pub enum DomainError {
    #[error(
        "Use a DNS name with letters, digits or hyphens. Wildcards and IP addresses are not allowed."
    )]
    Syntax,
    #[error(
        "Use .test, .localhost, .internal, .home.arpa, .example or .invalid. Public domains and .local are not allowed."
    )]
    Reserved,
}

/// The same local trust policy enforced by the CLI's CA trust boundary.
pub fn parse(input: &str) -> Result<String, DomainError> {
    let input = input.trim().to_ascii_lowercase();
    let domain = input.strip_suffix('.').unwrap_or(&input);
    if domain.len() > 253
        || !domain.split('.').all(|label| {
            !label.is_empty()
                && label.len() <= 63
                && label
                    .as_bytes()
                    .first()
                    .is_some_and(u8::is_ascii_alphanumeric)
                && label
                    .as_bytes()
                    .last()
                    .is_some_and(u8::is_ascii_alphanumeric)
                && label
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'-')
        })
    {
        return Err(DomainError::Syntax);
    }
    if !RESERVED
        .iter()
        .any(|suffix| domain == *suffix || domain.ends_with(&format!(".{suffix}")))
    {
        return Err(DomainError::Reserved);
    }
    Ok(domain.to_owned())
}
