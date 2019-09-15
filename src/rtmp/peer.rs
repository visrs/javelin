#[allow(unused_imports)]
use log::{error, debug, info};
use futures::{
    sync::mpsc,
    try_ready,
};
use tokio::prelude::*;
use bytes::{Bytes, BytesMut, BufMut};
use snafu::{Snafu, ResultExt};
use crate::{
    shared::Shared,
    bytes_stream::{self, BytesStream},
};
use super::{
    proto::{
        Session,
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
    session: Session,
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
            session: Session::new(session_config),
        }
    }

//    fn handle_incoming_bytes(&mut self) -> Result<()> {
//        let data = self.buffer.take();
//
//        let event_results = self.event_handler.handle(&data).context(EventHandlerError)?;
//
//        for result in event_results {
//            match result {
//                EventResult::Outbound(target_peer_id, packet) => {
//                    let peers = self.shared.peers.read();
//                    let peer = peers.get(&target_peer_id).unwrap();
//                    // debug!("Packet from {} to {} with {:?} bytes", self.id, target_peer_id, packet.bytes.len());
//                    peer.unbounded_send(Message::Raw(Bytes::from(packet.bytes))).unwrap();
//                },
//                EventResult::Disconnect => {
//                    self.disconnecting = true;
//                    break;
//                }
//            }
//        }
//
//        Ok(())
//    }
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
                for result in self.session.handle_bytes(&data).unwrap() {
                    match result {
                        SessionMessage::Packet { payload, .. } => {
                            self.sender.unbounded_send(Message::Raw(payload)).unwrap();
                        },
                        SessionMessage::RegisterSource(app_name) => {
//                            self.sender.unbounded_send(Message::RegisterSource(app_name)).unwrap();
                            debug!("Registering source for {}", app_name);
                        },
                        SessionMessage::Metadata(metadata) => {
                            debug!("Metadata: {:#?}", metadata);
                        },
                        SessionMessage::AudioData(bytes, timestamp) | SessionMessage::VideoData(bytes, timestamp) => {
                            debug!("Received multimedia data");
                        }
                        msg => debug!("Unhandled message: {:?}", msg),
                    }
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
