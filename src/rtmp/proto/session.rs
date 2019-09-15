use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
    rc::Rc,
};
#[allow(unused_imports)]
use log::{log, debug, info};
use snafu::{ensure, ResultExt, OptionExt};
use uuid::Uuid;
use rml_rtmp::{
    sessions::{self, ServerSessionResult, ServerSessionEvent},
    handshake::{Handshake, HandshakeProcessResult, PeerType},
};
use super::{
    message::*,
    event::{
        EventHandler,
        types::*,
    },
    error::{
        Result,
        SessionNotInitialized,
        SessionNotPlaying,
        SessionCreationFailed,
        HandshakeFailed,
        InvalidInput,
        RequestRejected,
        EmptyApplicationName,
        UnknownApplication,
        UnpermittedStreamKey,
    }
};


macro_rules! session_log {
    ($session:ident, $level:expr, $fmt:expr) => {
        session_log!($session, $level, $fmt,)
    };
    ($session:ident, $level:expr, $fmt:expr, $($arg:tt)*) => {
        log!($level, concat!("RTMP Session {}: ", $fmt), $session.id, $($arg)*)
    };
}

macro_rules! session_info {
    ($session:ident, $($arg:tt)+) => { session_log!($session, log::Level::Info, $($arg)+) }
}

macro_rules! session_debug {
    ($session:ident, $($arg:tt)+) => { session_log!($session, log::Level::Debug, $($arg)*) };
}


enum State {
    HandshakePending,
    Initialized,
    Publishing(ApplicationName, StreamKey),
    Playing(StreamId),
    Finished,
}


#[derive(Debug)]
pub struct Config {
    pub app_names: HashSet<ApplicationName>,
    pub stream_keys: HashMap<ApplicationName, StreamKey>,
}

impl Config {
    pub fn new(app_names: HashMap<ApplicationName, StreamKey>) -> Self {
        Self {
            app_names: app_names.keys().map(ToString::to_string).collect(),
            stream_keys: app_names,
        }
    }
}


pub struct Session {
    id: Uuid,
    state: State,
    handshake: Handshake,
    session: Option<sessions::ServerSession>,
    return_queue: Vec<Message>,
    config: Config,
}

impl Session {
    pub fn new(config: Config) -> Self {
        Self {
            id: Uuid::new_v4(),
            state: State::HandshakePending,
            handshake: Handshake::new(PeerType::Server),
            session: None,
            return_queue: Vec::new(),
            config,
        }
    }

    pub fn handle_bytes(&mut self, bytes: &[u8]) -> Result<Vec<Message>> {
        match self.state {
            State::HandshakePending => {
                self.perform_handshake(bytes)?;
            },
            _ => {
                self.handle_input(bytes)?;
            },
        }

        Ok(self.return_queue.drain(..).collect())
    }

    /// Handle a message produced from a different RTMP connection.
    pub fn handle_message(&mut self, message: Message) -> Result<Vec<Message>> {
        let stream_id = self.stream_id().context(SessionNotPlaying)?;

        match message {
            Message::VideoData(bytes, timestamp) => {
                self.handle_event(SendVideoData { payload: bytes, stream_id, timestamp })?;
            },
            Message::AudioData(bytes, timestamp) => {
                self.handle_event(SendAudioData { payload: bytes, stream_id, timestamp })?;
            },
            Message::Metadata(metadata) => {
                self.handle_event(SendMetadata { metadata, stream_id })?;
            },
            _ => (),
        }

        Ok(self.return_queue.drain(..).collect())
    }

    fn is_playing(&self) -> bool {
        self.stream_id().is_some()
    }

    fn stream_id(&self) -> Option<u32> {
        match self.state {
            State::Playing(id) => Some(id),
            _ => None,
        }
    }

    fn session(&mut self) -> Result<&mut sessions::ServerSession> {
        Ok(self.session.as_mut().context(SessionNotInitialized)?)
    }

    fn perform_handshake(&mut self, bytes: &[u8]) -> Result<()> {
        match self.handshake.process_bytes(&bytes).context(HandshakeFailed)? {
            HandshakeProcessResult::InProgress { response_bytes } => {
                self.return_queue.push(Message::from(response_bytes));
            },
            HandshakeProcessResult::Completed { response_bytes, remaining_bytes } => {
                if !response_bytes.is_empty() {
                    self.return_queue.push(Message::from(response_bytes));
                }

                self.initialize_session()?;

                if !remaining_bytes.is_empty() {
                    self.handle_input(&remaining_bytes)?;
                }

                self.state = State::Initialized;
            },
        }

        Ok(())
    }

    fn initialize_session(&mut self) -> Result<()> {
        let session_config = sessions::ServerSessionConfig::new();
        let (session, results) = sessions::ServerSession::new(session_config)
            .context(SessionCreationFailed)?;
        self.handle_results(results)?;
        self.session = Some(session);
        Ok(())
    }

    fn handle_input(&mut self, bytes: &[u8]) -> Result<()> {
        let results = {
            let mut session = self.session()?;
            session.handle_input(bytes).context(InvalidInput)?
        };
        self.handle_results(results)?;

        Ok(())
    }

    fn handle_results(&mut self, results: Vec<ServerSessionResult>) -> Result<()> {
        for result in results {
            match result {
                ServerSessionResult::OutboundResponse(packet) => {
                    self.return_queue.push(Message::from(packet));
                },
                ServerSessionResult::RaisedEvent(event) => {
                    self.delegate_event_handling(event)?;
                },
                ServerSessionResult::UnhandleableMessageReceived(_) => (),
            }
        }

        Ok(())
    }

    fn delegate_event_handling(&mut self, event: ServerSessionEvent) -> Result<()> {
        match event {
            ServerSessionEvent::ConnectionRequested { request_id, app_name } => {
                self.handle_event(ConnectionRequested { id: request_id, app_name })?;
            },
            ServerSessionEvent::PublishStreamRequested { request_id, app_name, stream_key, .. } => {
                self.handle_event(PublishingRequested { id: request_id, app_name, stream_key })?;
            },
            ServerSessionEvent::PublishStreamFinished { app_name, stream_key } => {
                self.handle_event(PublishingFinished { app_name, stream_key })?;
            },
            ServerSessionEvent::PlayStreamRequested { request_id, app_name, stream_id, .. } => {
                self.handle_event(PlaybackRequested { id: request_id, app_name, stream_id })?;
            },
            ServerSessionEvent::PlayStreamFinished { app_name, stream_key } => {
                self.handle_event(PlaybackFinished { app_name, stream_key })?;
            },
            ServerSessionEvent::StreamMetadataChanged { app_name, metadata, .. } => {
                self.handle_event(MetadataReceived { app_name, metadata })?;
            },
            ServerSessionEvent::AudioDataReceived { app_name, stream_key, timestamp, data } => {
                self.handle_event(AudioDataReceived { payload: data, app_name, stream_key, timestamp })?;
            },
            ServerSessionEvent::VideoDataReceived { app_name, stream_key, timestamp, data } => {
                self.handle_event(VideoDataReceived { payload: data, app_name, stream_key, timestamp })?;
            }
            ServerSessionEvent::AcknowledgementReceived { .. } => {
                session_debug!(self, "Acknowledgement received");
            },
            ev => {
                debug!("Unhandled RTMP event {:?}", ev);
            }
        }

        Ok(())
    }

    /// Accept an incoming request.
    fn accept_request(&mut self, id: u32) -> Result<()> {
        let session = self.session()?;
        let results = session.accept_request(id).context(RequestRejected)?;
        self.handle_results(results)?;
        Ok(())
    }

    /// Check if request with `app_name` can start publishing with `stream_key`.
    fn can_publish(&self, app_name: &str, stream_key: &str) -> Result<()> {
        if app_name.is_empty() {
            return EmptyApplicationName.fail();
        }

        match self.config.stream_keys.get(app_name) {
            None => UnknownApplication { app_name }.fail(),
            Some(k) => {
                if k != stream_key {
                    UnpermittedStreamKey { app_name, stream_key }.fail()
                } else {
                    Ok(())
                }
            },
        }
    }
}

impl EventHandler<ConnectionRequested> for Session {
    fn handle_event(&mut self, event: ConnectionRequested) -> Result<()> {
        session_debug!(self, "Connection request for app '{}'", event.app_name);

        ensure!(!event.app_name.is_empty(), EmptyApplicationName);

        self.accept_request(event.id)?;

        Ok(())
    }
}

impl EventHandler<PublishingRequested> for Session {
    fn handle_event(&mut self, event: PublishingRequested) -> Result<()> {
        session_info!(self, "Publishing request to app '{}' using stream key {}", event.app_name, event.stream_key);

        self.can_publish(&event.app_name, &event.stream_key)?;

        session_info!(self, "Publishing request to app '{}' granted", event.app_name);

        self.return_queue.push(Message::RegisterSource(event.app_name.clone()));
        self.accept_request(event.id)?;

        self.state = State::Publishing(event.app_name, event.stream_key);

        Ok(())
    }
}

impl EventHandler<PublishingFinished> for Session {
    fn handle_event(&mut self, event: PublishingFinished) -> Result<()> {
        session_info!(self, "Publishing of app '{}' finished", event.app_name);

        self.return_queue.push(Message::Finished);
        self.state = State::Finished;

        Ok(())
    }
}

impl EventHandler<PlaybackRequested> for Session {
    fn handle_event(&mut self, event: PlaybackRequested) -> Result<()> {
        session_info!(self, "Playback request of app '{}'", event.app_name);

        self.accept_request(event.id)?;

        self.return_queue.push(Message::RegisterSink(event.app_name));

        self.state = State::Playing(event.stream_id);

        Ok(())
    }
}

impl EventHandler<PlaybackFinished> for Session {
    fn handle_event(&mut self, event: PlaybackFinished) -> Result<()> {
        session_info!(self, "Playback of app '{}' finished", event.app_name);

        self.return_queue.push(Message::Finished);
        self.state = State::Finished;

        Ok(())
    }
}

impl EventHandler<MetadataReceived> for Session {
    fn handle_event(&mut self, event: MetadataReceived) -> Result<()> {
        session_debug!(self, "Metadata received");

        self.return_queue.push(Message::Metadata(event.metadata.into()));

        Ok(())
    }
}

impl EventHandler<VideoDataReceived> for Session {
    fn handle_event(&mut self, event: VideoDataReceived) -> Result<()> {
        // session_debug!(self, "Video data received");

        // self.session()?.send_audio_data(event., bytes.clone(), timestamp.clone(), true)

        Ok(())
    }
}

impl EventHandler<AudioDataReceived> for Session {
    fn handle_event(&mut self, event: AudioDataReceived) -> Result<()> {
        // session_debug!(self, "Audio data received");

        Ok(())
    }
}


impl EventHandler<SendVideoData> for Session {
   fn handle_event(&mut self, event: SendVideoData) -> Result<()> {
       let packet = self.session()?
           .send_video_data(event.stream_id, event.payload, event.timestamp, false)
           .unwrap();

       self.return_queue.push(packet.into());

       Ok(())
   }
}

impl EventHandler<SendAudioData> for Session {
    fn handle_event(&mut self, event: SendAudioData) -> Result<()> {
        let packet = self.session()?
            .send_audio_data(event.stream_id, event.payload, event.timestamp, false)
            .unwrap();

        self.return_queue.push(packet.into());

        Ok(())
    }
}

impl EventHandler<SendMetadata> for Session {
    fn handle_event(&mut self, event: SendMetadata) -> Result<()> {
        let packet = self.session()?
            .send_metadata(event.stream_id, Rc::new(event.metadata))
            .unwrap();

        self.return_queue.push(packet.into());

        Ok(())
    }
}
