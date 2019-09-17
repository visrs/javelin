use {
    std::{
        collections::{HashMap, HashSet},
        sync::Arc,
        rc::Rc,
    },
    log::{log, debug, info},
    snafu::{ensure, ResultExt, OptionExt},
    rml_rtmp::{
        sessions::{self, ServerSessionResult, ServerSessionEvent},
        handshake::{Handshake, HandshakeProcessResult, PeerType},
    },
};
use super::{
    message::*,
    event::{EventHandler, types::*},
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


pub struct Protocol {
    state: State,
    handshake: Handshake,
    session: Option<sessions::ServerSession>,
    return_queue: Vec<Message>,
    config: Config,
}

impl Protocol {
    pub fn new(config: Config) -> Self {
        Self {
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

    pub fn handle_message(&mut self, message: Message) -> Result<Vec<u8>> {
        let stream_id = self.stream_id().unwrap();

        match message {
            Message::AudioData(payload, timestamp) => {
                let packet = self.session()?
                    .send_audio_data(stream_id, payload, timestamp, false)
                    .unwrap();
                Ok(packet.bytes.into())
            },
            Message::VideoData(payload, timestamp) => {
                let packet = self.session()?
                    .send_video_data(stream_id, payload, timestamp, false)
                    .unwrap();
                Ok(packet.bytes.into())
            },
            Message::Metadata(metadata) => {
                let packet = self.session()?
                    .send_metadata(stream_id, Rc::new(metadata))
                    .unwrap();
                Ok(packet.bytes.into())
            },
            _ => {
                // FIXME: Should return error instead
                Ok(Vec::new())
            },
        }
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

    fn accept_request(&mut self, id: u32) -> Result<()> {
        let session = self.session()?;
        let results = session.accept_request(id).context(RequestRejected)?;
        self.handle_results(results)?;
        Ok(())
    }

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
                    self.handle_event(event)?;
                },
                ServerSessionResult::UnhandleableMessageReceived(_) => (),
            }
        }

        Ok(())
    }

    fn handle_event(&mut self, event: ServerSessionEvent) -> Result<()> {
        match event {
            ServerSessionEvent::ConnectionRequested { request_id, .. } => {
                self.accept_request(request_id)?;
            },
            ServerSessionEvent::PublishStreamRequested { request_id, app_name, stream_key, .. } => {
                self.return_queue.push(Message::Authenticate(app_name.clone(), stream_key.clone()));
                self.return_queue.push(Message::RegisterSource(app_name.clone()));
                self.accept_request(request_id)?;
                self.state = State::Publishing(app_name, stream_key);
            },
            ServerSessionEvent::PlayStreamRequested { request_id, app_name, stream_id, .. } => {
                self.return_queue.push(Message::RegisterSink(app_name));
                self.accept_request(request_id)?;
                self.state = State::Playing(stream_id);
            },
            ServerSessionEvent::PlayStreamFinished { app_name, stream_key } |
            ServerSessionEvent::PublishStreamFinished { app_name, stream_key } => {
                self.return_queue.push(Message::Finished(app_name));
                self.state = State::Finished;
            },
            ServerSessionEvent::StreamMetadataChanged { metadata, .. } => {
                self.return_queue.push(Message::Metadata(metadata.into()));
            },
            ServerSessionEvent::AudioDataReceived { timestamp, data, .. } => {
                self.return_queue.push(Message::AudioData(data, timestamp));
            },
            ServerSessionEvent::VideoDataReceived { timestamp, data, .. } => {
                self.return_queue.push(Message::VideoData(data, timestamp));
            }
            ServerSessionEvent::ReleaseStreamRequested { app_name, stream_key, .. } => {
                debug!("Release stream request: {:?}", stream_key);
                self.return_queue.push(Message::Authenticate(app_name, stream_key));
            },
            ServerSessionEvent::UnhandleableAmf0Command { command_name, mut additional_values, .. } => {
                if command_name == "releaseStream" {
                    let stream_key = additional_values.pop().unwrap().get_string().unwrap();
                    debug!("Release stream request: {:?}", stream_key);
                } else {
                    debug!("Unhandled RTMP command: {:?}", command_name);
                }
            },
            ServerSessionEvent::AcknowledgementReceived { .. } => {
                debug!("Acknowledgement received");
            },
            ServerSessionEvent::ClientChunkSizeChanged { .. } => {},
            ServerSessionEvent::PingResponseReceived { timestamp } => {
                debug!("Ping response: {:?}", timestamp);
            },
        }

        Ok(())
    }
}
