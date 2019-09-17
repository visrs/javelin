use futures::{
    try_ready,
    sync::{oneshot, mpsc},
    Future,
    Async,
};
use snafu::{Snafu, OptionExt, ResultExt};


pub type Response = Result<Message, Error>;
pub type ResponseReceiver = oneshot::Receiver<Response>;
pub type ResponseSender = oneshot::Sender<Response>;


#[derive(Debug, Snafu)]
pub enum Error {
    RequestFailed { source: mpsc::SendError<Message> },
    ResponseFailed { source: oneshot::Canceled },
}


#[derive(Debug, Clone)]
pub enum Message {
    RegisterSource { name: String },
    RegisterSink { name: String },
    Ping,
    Pong,
}

struct Outbox {
    sender: mpsc::UnboundedSender<Request>,
}

impl Outbox {
    pub fn send_message(&mut self, message: Message) -> Result<ResponseReceiver, Error> {
        let (request, receiver) = Request::channel(message);
        self.sender.unbounded_send(request).context(RequestFailed)?;
        Ok(receiver)
    }
}

// let (request, inbox) = Request::channel(Message::Ping);
// bus_sender.send(request);
// inbox.poll();
pub struct Request {
    pub message: Message,
    response_sender: ResponseSender
}

impl Request {
    pub fn channel(message: Message) -> (Self, Inbox) {
        let (response_sender, response_receiver) = oneshot::channel();
        (Self { message, response_sender }, response_receiver.into())
    }

    pub fn respond(self, response: Result<M, Error>) -> Result<(), Error> {
        self.response_sender.send(response).context(SendFailed)
    }
}
