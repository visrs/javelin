use std::{
    error::Error as StdError,
    fmt::{self, Display, Formatter},
};
use snafu::Snafu;
use rml_rtmp::{
    sessions::ServerSessionError,
    handshake::HandshakeError,
};


#[derive(Debug)]
pub enum RtmpError {
    ServerSessionError(ServerSessionError),
    HandshakeError(HandshakeError),
}

impl StdError for RtmpError {}

impl Display for RtmpError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let s = match self {
            Self::ServerSessionError(inner) => inner.to_string(),
            Self::HandshakeError(inner) => inner.to_string(),
        };
        write!(f, "{}", s)
    }
}

impl From<ServerSessionError> for RtmpError {
    fn from(val: ServerSessionError) -> Self {
        RtmpError::ServerSessionError(val)
    }
}

impl From<HandshakeError> for RtmpError {
    fn from(val: HandshakeError) -> Self {
        RtmpError::HandshakeError(val)
    }
}


#[derive(Debug, Snafu)]
#[snafu(visibility(pub(super)))]
pub enum Error {
    #[snafu(display("RTMP session was not initialized"))]
    SessionNotInitialized,

    #[snafu(display("Session received external message, but is not in playing state"))]
    SessionNotPlaying,

    #[snafu(display("RTMP application name was empty"))]
    EmptyApplicationName,

    #[snafu(display("No RTMP application with name '{}' registered", app_name))]
    UnknownApplication { app_name: String },

    #[snafu(display("Key '{}' not permitted for application '{}'", stream_key, app_name))]
    UnpermittedStreamKey { app_name: String, stream_key: String },

    #[snafu(display("RTMP handshake failed: {}", source))]
    HandshakeFailed {
        #[snafu(source(from(HandshakeError, RtmpError::from)))]
        source: RtmpError
    },

    #[snafu(display("RTMP session creation failed: {}", source))]
    SessionCreationFailed {
        #[snafu(source(from(ServerSessionError, RtmpError::from)))]
        source: RtmpError
    },

    #[snafu(display("Invalid input: {}", source))]
    InvalidInput {
        #[snafu(source(from(ServerSessionError, RtmpError::from)))]
        source: RtmpError
    },

    #[snafu(display("Request rejected: {}", source))]
    RequestRejected {
        #[snafu(source(from(ServerSessionError, RtmpError::from)))]
        source: RtmpError
    },
}


pub(super) type Result<T, E = Error> = std::result::Result<T, E>;
