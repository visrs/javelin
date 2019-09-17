use futures::{
    try_ready,
    sync::{mpsc, oneshot},
    Future,
    Stream,
    Async,
};
use super::{
    message::{Request, Message}
};


type RequestSender = mpsc::UnboundedSender<Request<Message>>;
type RequestReceiver = mpsc::UnboundedReceiver<Request<Message>>;


pub struct Config {
}


pub struct SessionManager {
    config: Config,
    sender: RequestSender,
    receiver: RequestReceiver,
}

impl SessionManager {
    pub fn new(config: Config) -> Self {
        let (sender, receiver) = mpsc::unbounded();
        Self { config, sender, receiver }
    }

    pub fn sender(&self) -> RequestSender {
        self.sender.clone()
    }

    fn handle_request(&self, request: Request<Message>) -> Result<(), ()> {
        match request.message {
            Message::RegisterSource { name } => {
                request.respond(Ok(Message::Pong));
                Ok(())
            },
            _ => Ok(()),
        }

        Ok(())
    }
}

impl Future for SessionManager {
    type Item = ();
    type Error = ();

    fn poll(&mut self) -> Result<Async<Self::Item>, Self::Error> {
        while let Some(message) = try_ready!(self.receiver.poll()) {

        }

        Ok(Async::Ready(()))
    }
}


pub struct Session {
    publisher:
}
