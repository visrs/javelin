use tokio::{
    prelude::*,
};
use super::{
    protocol::{Protocol, Config},
    message::Message,
};
use crate::{
    bytes_stream::BytesStream,
};


pub struct Connection<S>
    where S: AsyncRead + AsyncWrite
{
    protocol: Protocol,
    bstream: BytesStream<S>,
}

impl<S> Connection<S>
    where S: AsyncRead + AsyncWrite
{
    pub fn new(socket: S, config: Config) -> Self {
        Self {
            protocol: Protocol::new(config),
            bstream: BytesStream::new(socket),
        }
    }

    fn handle_message(&mut self, message: Message) {
        match message {
            Message::RegisterSource(app_name) => {
                // send event to session manager
            },
            Message::Packet { droppable: _, payload: _ } => {}
            Message::RegisterSink(_) => {}
            Message::VideoData(_, _) => {}
            Message::AudioData(_, _) => {}
            Message::Metadata(_) => {}
            Message::Finished => {}
        }
    }
}

impl<S> Future for Connection<S>
    where S: AsyncRead + AsyncWrite
{
    type Item = ();
    type Error = ();

    fn poll(&mut self) -> Result<Async<Self::Item>, Self::Error> {
        // handle incoming bytes
        if let Some(bytes) = try_ready!(self.bstream.poll()) {
            for message in self.protocol.handle_bytes(&bytes).unwrap() {
            }
        }

        Ok(Async::Ready(()))
    }
}
