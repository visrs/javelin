mod error;
mod message;
mod mailbox;


use {
    std::fmt::Debug,
    log::debug,
    snafu::{ResultExt, OptionExt},
    futures::{
        prelude::*,
        try_ready,
        sync::{mpsc, oneshot}
    },
    bytes::Bytes,
};
use self::{
    error::*,
};
pub use self::{
    mailbox::Mailbox,
    error::Error,
    message::{Message, ResponseReceiver}
};


pub struct BusSender<T, R>
    where R: Send
{
    sender: mpsc::UnboundedSender<Message<T, R>>,
}

impl<T, R> Clone for BusSender<T, R>
    where R: Send
{
    fn clone(&self) -> Self {
        Self { sender: self.sender.clone() }
    }
}

impl<T, R> BusSender<T, R>
    where R: Send
{
    pub fn send_message(&mut self, message: Message<T, R>) -> Result<(), Error> {
        self.sender.unbounded_send(message).ok().context(BusSendFailed)?;
        Ok(())
    }

    pub fn send(&mut self, inner: T) -> Result<ResponseReceiver<R>, Error> {
        let (sender, receiver) = oneshot::channel();
        let message = Message::new(inner, sender);
        self.send_message(message)?;
        Ok(receiver)
    }
}


pub struct Bus<T, R>
    where R: Send
{
    sender: mpsc::UnboundedSender<Message<T, R>>,
    receiver: mpsc::UnboundedReceiver<Message<T, R>>,
}

impl<T, R> Bus<T, R>
    where R: Send
{
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::unbounded();
        Self { sender, receiver }
    }

    pub fn sender(&self) -> BusSender<T, R> {
        BusSender { sender: self.sender.clone() }
    }

    pub fn recv(&mut self) -> Result<Async<Option<Message<T, R>>>, ()> {
        self.receiver.poll()
    }
}
