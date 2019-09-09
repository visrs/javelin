use std::{
    error::Error as StdError,
    fmt::{self, Display, Formatter},
};
use rml_rtmp::{
    sessions::ServerSessionError,
    handshake::HandshakeError,
};


/// Wrapper for external error type
#[derive(Debug)]
pub enum Error {
    ServerSession(ServerSessionError),
    Handshake(HandshakeError),
}

impl StdError for Error {}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let s = match self {
            Self::ServerSession(inner) => inner.to_string(),
            Self::Handshake(inner) => inner.to_string(),
        };
        write!(f, "{}", s)
    }
}

impl From<ServerSessionError> for Error {
    fn from(val: ServerSessionError) -> Self {
        Self::ServerSession(val)
    }
}

impl From<HandshakeError> for Error {
    fn from(val: HandshakeError) -> Self {
        Self::Handshake(val)
    }
}
