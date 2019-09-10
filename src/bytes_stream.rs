use tokio::{
    prelude::*,
    io,
};
use futures::{try_ready, StartSend};
use bytes::{Bytes, BytesMut, BufMut};
use snafu::{ensure, Snafu, ResultExt};


#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("Read buffer was full"))]
    ReadBufferFull,

    #[snafu(display("Write buffer was full"))]
    WriteBufferFull,

    #[snafu(display("Wrote 0 bytes while socket was ready"))]
    InvalidWrite,

    #[snafu(display("Reading from the socket failed: {}", source))]
    ReadFailed { source: io::Error },

    #[snafu(display("Writing to the socket failed: {}", source))]
    WriteFailed { source: io::Error },
}


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

    fn fill_read_buffer(&mut self) -> Poll<(), Error> {
        ensure!(self.buf_in.len() < std::usize::MAX - 4096, ReadBufferFull);

        loop {
            self.buf_in.reserve(4096);
            let bytes_read = try_ready!(self.socket.read_buf(&mut self.buf_in).context(ReadFailed));

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
        ensure!(self.buf_out.len() < std::usize::MAX, WriteBufferFull);

        self.buf_out.reserve(item.len());
        self.buf_out.put(item);
        Ok(AsyncSink::Ready)
    }

    fn poll_complete(&mut self) -> Poll<(), Self::SinkError> {
        let bytes_written = try_ready!(self.socket.poll_write(&self.buf_out).context(WriteFailed));

        ensure!(bytes_written > 0, InvalidWrite);

        let _ = self.buf_out.split_to(bytes_written);

        if self.buf_out.is_empty() {
            Ok(Async::Ready(()))
        } else {
            Ok(Async::NotReady)
        }
    }
}
