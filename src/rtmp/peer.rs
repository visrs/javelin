use {
    log::{error, debug, info},
    futures::{
        sync::mpsc,
        try_ready,
    },
    tokio::prelude::*,
    bytes::{Bytes, BytesMut, BufMut},
    snafu::{Snafu, ResultExt},
};
use crate::{
    shared::Shared,
    bytes_stream::{self, BytesStream},
};
use super::{
    proto::{
        Protocol,
        Config as SessionConfig,
        Message as SessionMessage,
    },
};


#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("Bytes Stream: {}", source))]
    BytesStreamError { source: bytes_stream::Error },
}

type Result<T, E = Error> = std::result::Result<T, E>;


pub enum Message {
    Raw(Bytes),
    Disconnect,
}

pub type Sender = mpsc::UnboundedSender<Message>;
type Receiver = mpsc::UnboundedReceiver<Message>;


/// Represents an incoming connection
pub struct Peer<S>
    where S: AsyncRead + AsyncWrite
{
    id: u64,
    bytes_stream: BytesStream<S>,
    sender: Sender,
    receiver: Receiver,
    shared: Shared,
    disconnecting: bool,
    session: Protocol,
}

impl<S> Peer<S>
    where S: AsyncRead + AsyncWrite
{
    pub fn new(id: u64, bytes_stream: BytesStream<S>, shared: Shared) -> Self {
        let (sender, receiver) = mpsc::unbounded();

        {
            let mut peers = shared.peers.write();
            peers.insert(id, sender.clone());
        }

        // TODO: refactor config handling
        let app_names = shared.config.read().permitted_stream_keys.clone();
        let session_config = SessionConfig::new(app_names);

        Self {
            id,
            bytes_stream,
            sender,
            receiver,
            shared,
            disconnecting: false,
            session: Protocol::new(session_config),
        }
    }
}

impl<S> Drop for Peer<S>
    where S: AsyncRead + AsyncWrite
{
    fn drop(&mut self) {
        let mut peers = self.shared.peers.write();
        peers.remove(&self.id);

        info!("Closing connection: {}", self.id);
    }
}

impl<S> Future for Peer<S>
    where S: AsyncRead + AsyncWrite
{
    type Item = ();
    type Error = Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        while let Async::Ready(Some(msg)) = self.receiver.poll().unwrap() {
            match msg {
                Message::Raw(val) => {
                    (&mut self.bytes_stream)
                        .send(val).poll()
                        .expect("BytesStream send should be infallible");
                },
                Message::Disconnect => {
                    self.disconnecting = true;
                    break;
                }
            }
        }

        if let Some(data) = try_ready!(self.bytes_stream.poll().context(BytesStreamError)) {
            for message in self.session.handle_bytes(&data).unwrap() {
                match message {
                    SessionMessage::Packet { payload, .. } => {
                        self.sender.unbounded_send(Message::Raw(payload)).unwrap();
                    },
                    | SessionMessage::Metadata(..)
                    | SessionMessage::AudioData(..)
                    | SessionMessage::VideoData(..) => {},
                    msg => debug!("RTMP: {:?}", msg),
                }
            }
        } else {
            return Ok(Async::Ready(()));
        }

        if self.disconnecting {
            Ok(Async::Ready(()))
        } else {
            Ok(Async::NotReady)
        }
    }
}
