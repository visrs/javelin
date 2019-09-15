use std::collections::VecDeque;
use tokio::prelude::*;
use futures::try_ready;
use super::{
    message::{Message, Metadata, VideoData, AudioData},
    session::{Session, Config},
    error::Error,
};
use crate::{
    bytes_stream::BytesStream,
};


#[derive(Default)]
struct Cache {
    metadata: Option<Metadata>,
    video_sequence_header: Option<VideoData>,
    audio_sequence_header: Option<AudioData>,
}



struct RtmpStream<S>
    where S: AsyncRead + AsyncWrite
{
    session: Session,
    messages: VecDeque<Message>,
    socket: BytesStream<S>,
}

impl RtmpStream<S>
    where S: AsyncRead + AsyncWrite
{
    pub fn new(socket: S, config: Config) -> Self {
        Self {
            session: Session::new(config),
            messages: VecDeque::new(),
            socket,
        }
    }
}

impl Stream for RtmpStream<S>
    where S: AsyncRead + AsyncWrite
{
    type Item = Message;
    type Error = Error;

    fn poll(&mut self) -> Result<Async<Option<Self::Item>>, Self::Error> {
        if let Some(message) = self.messages.pop_back() {
            return Ok(Async::Ready(Some(message)));
        }

        if let Some(bytes) = try_ready!(self.socket.poll()) {
            if let Some(message) = self.session.handle_bytes(&bytes)?.split_first() {
                match message {
                    (head, &[]) => {
                        return Ok(Async::Ready(Some(head.clone())));
                    },
                    (head, tail) => {
                        self.messages.extend(tail);
                        return Ok(Async::Ready(Some(head.clone())));
                    }
                }
            }
        }

        Ok(Async::Ready(None))
    }
}
