use tokio::{
    prelude::*,
    io,
};
use futures::{try_ready, StartSend};
use bytes::{Bytes, BytesMut, BufMut};
use crate::error::Error;


pub struct BytesStream<S>
    where S: AsyncRead + AsyncWrite
{
    socket: S,
    buf_in: BytesMut,
    buf_out: BytesMut,
}

impl<S> BytesStream<S>
    where S: AsyncRead + AsyncWrite
{
    pub fn new(socket: S) -> Self
    {
        Self {
            socket,
            buf_in: BytesMut::new(),
            buf_out: BytesMut::new(),
        }
    }

    fn fill_read_buffer(&mut self) -> Poll<(), io::Error> {
        loop {
            self.buf_in.reserve(4096);
            let bytes_read = try_ready!(self.socket.read_buf(&mut self.buf_in));

            if bytes_read == 0 {
                return Ok(Async::Ready(()));
            }
        }
    }
}

impl<S> Stream for BytesStream<S>
    where S: AsyncRead + AsyncWrite
{
    type Item = Bytes;
    type Error = Error;

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        let is_socket_closed = self.fill_read_buffer()?.is_ready();

        let data = self.buf_in.take();
        if !data.is_empty() {
            return Ok(Async::Ready(Some(data.freeze())))
        }

        if is_socket_closed {
            // Stream is finished
            Ok(Async::Ready(None))
        } else {
            Ok(Async::NotReady)
        }
    }
}

impl<S> Sink for BytesStream<S>
    where S: AsyncWrite + AsyncRead
{
    type SinkItem = Bytes;
    type SinkError = Error;

    fn start_send(&mut self, item: Self::SinkItem) -> StartSend<Self::SinkItem, Self::SinkError> {
        // This could potentially panic if the buffer reaches `usize::MAX`.
        self.buf_out.reserve(item.len());
        self.buf_out.put(item);
        Ok(AsyncSink::Ready)
    }

    fn poll_complete(&mut self) -> Poll<(), Self::SinkError> {
        let bytes_written = try_ready!(self.socket.poll_write(&self.buf_out));

        if bytes_written == 0 {
            return Err(Error::from("Read 0 bytes while socket was ready"));
        }

        let _ = self.buf_out.split_to(bytes_written);

        if self.buf_out.is_empty() {
            Ok(Async::Ready(()))
        } else {
            Ok(Async::NotReady)
        }
    }
}
