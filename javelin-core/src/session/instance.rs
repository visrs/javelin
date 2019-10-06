use {
    log::debug,
    futures::{
        prelude::*,
        try_ready,
    },
    uuid::Uuid,
};
use crate::{
    utils::UnboundedQueue,
    bus,
};
use super::{
    common::{DataSender, DataReceiver},
};


pub(super) type BusSender = bus::BusSender<Event, ()>;
pub(super) type Bus = bus::Bus<Event, ()>;


#[derive(Debug)]
pub(super) enum Event {
    AddSink(DataSender),
    AddSource(DataReceiver),
    Release,
}


pub(super) struct Session {
    id: Uuid,
    bus: Bus,
    source: Option<DataReceiver>,
    sinks: UnboundedQueue<DataSender>,
}

impl Session {
    pub fn new() -> Self {
        let id = Uuid::new_v4();
        debug!("New session {}", id);
        Self {
            id,
            bus: Bus::new(),
            source: None,
            sinks: UnboundedQueue::new(),
        }
    }

    pub fn with_receiver(data_receiver: DataReceiver) -> Self {
        Self {
            source: Some(data_receiver),
            ..Self::new()
        }
    }

    pub fn sender(&self) -> BusSender {
        self.bus.sender()
    }

    fn poll_events(&mut self) -> Poll<(), ()> {
        while let Some(message) = try_ready!(self.bus.recv()) {
            match message.inner {
                Event::AddSink(sink) => {
                    debug!("Adding new sink to session {}", self.id);
                    self.sinks.push(sink);
                },
                Event::AddSource(src) => {
                    // Drop old receiver, effectively killing this connection.
                    self.source = Some(src);
                },
                _ => (),
            }
        }

        Ok(Async::Ready(()))
    }

    fn poll_source(&mut self) -> Poll<(), ()> {
        if let Some(source) = &mut self.source {
            while let Some(data) = try_ready!(source.poll()) {
                while let Some(sink) = self.sinks.pop() {
                    if sink.unbounded_send(data.clone()).is_ok() {
                        self.sinks.push(sink);
                    } else {
                        debug!("Removing sink from session {}", self.id);
                    }
                }
            }

            // Poll returned None, so the source stream is done.
            return Ok(Async::Ready(()))
        }

        // Don't signal ready if there's no source yet.
        Ok(Async::NotReady)
    }
}

impl Future for Session {
    type Item = ();
    type Error = ();

    fn poll(&mut self) -> Result<Async<Self::Item>, Self::Error> {
        if self.poll_source()?.is_ready() {
            // Source is done, so the session can be closed as well.
            return Ok(Async::Ready(()))
        };

        if self.poll_events()?.is_ready() {
            // Our bus connection is done, making this session invalid.
            return Ok(Async::Ready(()))
        };

        Ok(Async::NotReady)
    }
}
