use {
    std::collections::HashMap,
    log::debug,
    futures::{
        prelude::*,
        try_ready,
    },
};
use crate::{
    bus::{self, Bus, BusSender, Mailbox},
};
use super::{
    error::*,
    common::{DataReceiver, DataSender},
    instance::{
        Session,
        Event as InstanceEvent,
        BusSender as InstanceBusSender,
    },
};


pub type SessionBusSender = BusSender<Event, Response>;
pub type SessionMessage = bus::Message<Event, Response>;
pub type SessionMailbox = Mailbox<Event, Response>;


pub struct SessionManager {
    sessions: Registry,
    bus: Bus<Event, Response>,
    authenticator: Authenticator,
}

impl SessionManager {
    pub fn new(allowed_sessions: HashMap<String, String>) -> Self {
        Self {
            sessions: Registry::new(),
            bus: Bus::new(),
            authenticator: Authenticator::new(allowed_sessions),
        }
    }

    pub fn sender(&self) -> SessionBusSender {
        self.bus.sender()
    }

    fn handle_message(&mut self, mut message: SessionMessage) {
        match message.inner {
            Event::Authenticate(ref session_name, ref session_key) => {
                if self.authenticator.is_authenticated(session_name, session_key) {
                    message.response.send(Response::Accept(session_name.to_string()));
                } else {
                    message.response.send(Response::Reject);
                }
            },
            Event::RegisterSource(name, data_receiver) => {
                message.response.send(Response::MessageReceived);

                let session = Session::with_receiver(data_receiver);
                self.sessions.register(name, &session);
                tokio::spawn(session);
            },
            Event::RegisterSink(name, data_sender) => {
                message.response.send(Response::MessageReceived);

                self.sessions.notify(name.clone(), InstanceEvent::AddSink(data_sender));
            },
            ev => debug!("Received Event: {:?}", ev),
        }
    }
}

impl Future for SessionManager {
    type Item = ();
    type Error = ();

    fn poll(&mut self) -> Result<Async<Self::Item>, Self::Error> {
        while let Some(message) = try_ready!(self.bus.recv()) {
            self.handle_message(message);
        }

        Ok(Async::Ready(()))
    }
}


#[derive(Debug)]
pub enum Event {
    Authenticate(String, String),
    RegisterSource(String, DataReceiver),
    RegisterSink(String, DataSender),
    CloseSession(String),
}


#[derive(Debug)]
pub enum Response {
    Accept(String),
    Reject,
    MessageReceived,
}


#[derive(Default)]
struct Registry {
    entries: HashMap<String, InstanceBusSender>,
}

impl Registry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, name: String, session: &Session) {
        if self.entries.contains_key(&name) {
            // TODO: Handle active session according to setting
        } else {
            self.entries.insert(name, session.sender());
        }
    }

    fn replace(&mut self, name: String, session: &Session) {

    }

    pub fn notify<S>(&mut self, name: S, event: InstanceEvent)
        where S: AsRef<str>
    {
        if let Some(session) = self.entries.get_mut(name.as_ref()) {
            session.send(event);
        }
    }
}


struct Authenticator {
    items: HashMap<String, String>
}

impl Authenticator {
    pub fn new(items: HashMap<String, String>) -> Self {
        Self { items }
    }

    pub fn is_authenticated(&self, name: &str, token: &str) -> bool {
        match self.items.get(name) {
            Some(t) => token == t,
            None => false
        }
    }
}
