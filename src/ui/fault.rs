use crate::control::Error;

#[derive(Clone, Debug)]
pub enum Fault {
    Offline(String),
    SignedOut(String),
    /// The daemon may have acted, but no reply came back.
    Lost(String),
    Error(String),
}

impl From<Error> for Fault {
    fn from(error: Error) -> Self {
        let message = error.to_string();
        match error {
            Error::Io(error)
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::NotFound | std::io::ErrorKind::ConnectionRefused
                ) =>
            {
                Self::Offline(message)
            }
            Error::Io(_) | Error::Protocol(_) => Self::Lost(message),
            Error::Api { status: 401, .. } => Self::SignedOut(message),
            Error::Api {
                state: Some(state), ..
            } if state == "needs_sign_in" => Self::SignedOut(message),
            _ => Self::Error(message),
        }
    }
}
