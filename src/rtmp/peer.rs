#[allow(unused_imports)]
use log::{error, debug, info};
use futures::{
    sync::mpsc,
    try_ready,
};
use tokio::prelude::*;
use bytes::{Bytes, BytesMut, BufMut};
use rml_rtmp::{
    handshake::{
        HandshakeError,
        Handshake as RtmpHandshake,
        HandshakeProcessResult,
        PeerType,
    },
};
use snafu::{Snafu, ResultExt};
use crate::shared::Shared;
use super::{
    error::Error as RtmpError,
    event::{
        self,
        Handler as EventHandler,
        EventResult,
    },
    bytes_stream::{
        self,
        BytesStream,
    }
};


#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("Handshake for peer {} failed: {}", peer_id, source))]
    HandshakeFailed {
        #[snafu(source(from(HandshakeError, RtmpError::from)))]
        source: RtmpError,
        peer_id: u64,
    },

    #[snafu(display("Bytes Stream: {}", source))]
    BytesStreamError { source: bytes_stream::Error },

    #[snafu(display("Event Handler: {}", source))]
    EventHandlerError { source: event::Error },
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
    buffer: BytesMut,
    event_handler: EventHandler,
    disconnecting: bool,
    handshake_completed: bool,
    handshake: RtmpHandshake,
}

impl<S> Peer<S>
    where S: AsyncRead + AsyncWrite
{
    pub fn new(id: u64, bytes_stream: BytesStream<S>, shared: Shared) -> Self {
        let (sender, receiver) = mpsc::unbounded();
        let event_handler = EventHandler::new(id, shared.clone())
            .unwrap_or_else(|_| {
                panic!("Failed to create event handler for peer {}", id)
            });

        {
            let mut peers = shared.peers.write();
            peers.insert(id, sender.clone());
        }

        Self {
            id,
            bytes_stream,
            sender,
            receiver,
            shared,
            buffer: BytesMut::with_capacity(4096),
            event_handler,
            handshake_completed: false,
            disconnecting: false,
            handshake: RtmpHandshake::new(PeerType::Server),
        }
    }

    fn handle_handshake(&mut self) -> Poll<(), Error> {
        use self::HandshakeProcessResult as HandshakeState;

        let data = self.buffer.take().freeze();
        let handshake = self.handshake
            .process_bytes(&data)
            .context(HandshakeFailed { peer_id: self.id })?;

        let response_bytes = match handshake {
            HandshakeState::InProgress { response_bytes } => {
                debug!("Handshake pending...");
                response_bytes
            },
            HandshakeState::Completed { response_bytes, remaining_bytes } => {
                info!("Handshake for client {} successful", self.id);
                debug!("Remaining bytes after handshake: {}", remaining_bytes.len());
                self.handshake_completed = true;

                if !remaining_bytes.is_empty() {
                    self.buffer.reserve(remaining_bytes.len());
                    self.buffer.put(remaining_bytes);
                    self.handle_incoming_bytes()?;
                }

                response_bytes
            }
        };

        if !response_bytes.is_empty() {
            self.sender.unbounded_send(Message::Raw(Bytes::from(response_bytes))).unwrap();
        }

        Ok(Async::Ready(()))
    }

    fn handle_incoming_bytes(&mut self) -> Result<()> {
        let data = self.buffer.take();

        let event_results = self.event_handler.handle(&data).context(EventHandlerError)?;

        for result in event_results {
            match result {
                EventResult::Outbound(target_peer_id, packet) => {
                    let peers = self.shared.peers.read();
                    let peer = peers.get(&target_peer_id).unwrap();
                    // debug!("Packet from {} to {} with {:?} bytes", self.id, target_peer_id, packet.bytes.len());
                    peer.unbounded_send(Message::Raw(Bytes::from(packet.bytes))).unwrap();
                },
                EventResult::Disconnect => {
                    self.disconnecting = true;
                    break;
                }
            }
        }

        Ok(())
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

        match try_ready!(self.bytes_stream.poll().context(BytesStreamError)) {
            Some(data) => {
                self.buffer.reserve(data.len());
                self.buffer.put(data);

                if self.handshake_completed {
                    self.handle_incoming_bytes()?;
                } else {
                    try_ready!(self.handle_handshake());
                }
            },
            None => {
                return Ok(Async::Ready(()));
            },
        }

        if self.disconnecting {
            Ok(Async::Ready(()))
        } else {
            Ok(Async::NotReady)
        }
    }
}
