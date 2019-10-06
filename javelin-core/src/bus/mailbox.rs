use {
    std::{
        collections::VecDeque,
        fmt::Debug,
    },
    futures::{
        prelude::*,
        sync::{mpsc, oneshot},
    },
    snafu::ResultExt,
};
use crate::{
    utils::UnboundedQueue,
};
use super::{
    error::*,
    message::ResponseReceiver,
    BusSender,
};



pub struct Mailbox<T, R>
    where R: Send
{
    sender: BusSender<T, R>,
    receivers: UnboundedQueue<ResponseReceiver<R>>,
    outgoing: Vec<T>,
}

impl<T, R> Mailbox<T, R>
    where R: Send
{
    pub fn new(sender: BusSender<T, R>) -> Self {
        Self {
            sender,
            receivers: UnboundedQueue::new(),
            outgoing: Vec::new(),
        }
    }
}

impl<T, R> Stream for Mailbox<T, R>
    where R: Send
{
    type Item = R;
    type Error = Error;

    fn poll(&mut self) -> Result<Async<Option<Self::Item>>, Self::Error> {
        while let Some(mut receiver) = self.receivers.pop() {
            match receiver.poll().context(BusResponseFailed)? {
                Async::NotReady => self.receivers.push(receiver),
                Async::Ready(event) => return Ok(Async::Ready(Some(event))),
            }
        }

        Ok(Async::NotReady)
    }
}

impl<T, R> Sink for Mailbox<T, R>
    where R: Send
{
    type SinkItem = T;
    type SinkError = Error;

    fn start_send(&mut self, item: Self::SinkItem) -> Result<AsyncSink<Self::SinkItem>, Self::SinkError> {
        self.outgoing.push(item);
        Ok(AsyncSink::Ready)
    }

    fn poll_complete(&mut self) -> Result<Async<()>, Self::SinkError> {
        if let Some(msg) = self.outgoing.pop() {
            let receiver = self.sender.send(msg)?;
            self.receivers.push(receiver);
        }
        Ok(Async::Ready(()))
    }
}
