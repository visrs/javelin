use {
    std::fmt::{self, Debug},
    futures::sync::oneshot,
};


pub type ResponseSender<R> = oneshot::Sender<R>;
pub type ResponseReceiver<R> = oneshot::Receiver<R>;


pub struct Responder<R: Send>(ResponseSender<R>);

impl<R> Responder<R>
    where R: Send
{
    pub fn send(self, message: R) {
        if let Err(_) = self.0.send(message) {
            // TODO: Maybe handle this error in some way
        }
    }
}


pub struct Message<T, R>
    where R: Send
{
    pub inner: T,
    pub response: Responder<R>,
}

impl<T, R> Message<T, R>
    where R: Send
{
    pub fn new(inner: T, sender: ResponseSender<R>) -> Self {
        Self {
            inner,
            response: Responder(sender)
        }
    }
}

impl<T, R> Debug for Message<T, R>
    where R: Send,
          T: Debug
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.inner)
    }
}
