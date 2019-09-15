use rml_rtmp::sessions::{
    ServerSession,
    ServerSessionConfig,
    ServerSessionResult,
    ServerSessionError,
};
use snafu::{Snafu, ResultExt};
use super::error::Error as RtmpError;
use crate::{
    media::Channel,
    shared::Shared,
};


#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("Session request denied"))]
    RequestDenied {
        #[snafu(source(from(ServerSessionError, RtmpError::from)))]
        source: RtmpError,
    },

    #[snafu(display("Session creation failed"))]
    SessionCreationFailed {
        #[snafu(source(from(ServerSessionError, RtmpError::from)))]
        source: RtmpError,
    },
}

type Result<T, E = Error> = std::result::Result<T, E>;


#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClientState {
    Waiting,
    Publishing(String, String),
    Watching(String, u32),
}


/// Represents a session of a connected client
pub struct Client {
    peer_id: u64,
    state: ClientState,
    shared: Shared,
    pub session: ServerSession,
    pub received_video_keyframe: bool,
}

impl Client {
    #[allow(clippy::new_ret_no_self)]
    pub fn new(peer_id: u64, shared: Shared) -> Result<(Self, Vec<ServerSessionResult>)> {
        let session_config = ServerSessionConfig::new();
        let (session, results) = ServerSession::new(session_config).context(SessionCreationFailed)?;

        let this = Self {
            peer_id,
            shared,
            session,
            state: ClientState::Waiting,
            received_video_keyframe: false,
        };

        Ok((this, results))
    }

    pub fn accept_request(&mut self, request_id: u32) -> Result<Vec<ServerSessionResult>> {
        self.session.accept_request(request_id).context(RequestDenied)
    }

    pub fn publish(&mut self, channel: &mut Channel, app_name: String, stream_key: String) {
        channel.set_publisher(self.peer_id, stream_key.clone());
        self.state = ClientState::Publishing(app_name, stream_key);
    }

    pub fn watch(&mut self, channel: &mut Channel, stream_id: u32, app_name: String) {
        channel.add_watcher(self.peer_id);
        self.state = ClientState::Watching(app_name, stream_id);
    }

    pub fn watched_stream(&self) -> Option<u32> {
        match self.state {
            ClientState::Watching(_, stream_id) => Some(stream_id),
            _ => None,
        }
    }
}

impl Drop for Client {
    fn drop(&mut self) {
        if let ClientState::Watching(ref app_name, _) = self.state {
            let mut streams = self.shared.streams.write();
            if let Some(stream) = streams.get_mut(app_name) {
                stream.watchers.remove(&self.peer_id);
            }
        }
    }
}
