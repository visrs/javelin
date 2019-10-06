use {
    futures::{
        prelude::*,
        sync::mpsc,
    },
};


pub struct UnboundedQueue<T>
    where T: Send
{
    pusher: mpsc::UnboundedSender<T>,
    popper: mpsc::UnboundedReceiver<T>,
}

impl<T> UnboundedQueue<T>
    where T: Send
{
    pub fn new() -> Self {
        let (pusher, popper) = mpsc::unbounded();
        Self { pusher, popper }
    }

    pub fn push(&mut self, item: T) {
        self.pusher.unbounded_send(item).unwrap();
    }

    pub fn pop(&mut self) -> Option<T> {
        match self.popper.poll() {
            Ok(Async::Ready(rx)) => rx,
            _ => None
        }
    }
}

