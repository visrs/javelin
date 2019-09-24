use {
    std::{path::PathBuf, fs, time::{Instant, Duration}},
    log::{debug, error},
    futures::sync::mpsc::{self, UnboundedSender, UnboundedReceiver},
    tokio::{prelude::*, timer::DelayQueue},
};


type Batch = Vec<PathBuf>;
type Message = (Duration, Batch);
pub type Sender = UnboundedSender<Message>;
type Receiver = UnboundedReceiver<Message>;


pub struct FileCleaner {
    items: DelayQueue<Batch>,
    receiver: Receiver,
    sender: Sender,
}

impl FileCleaner {
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::unbounded();

        Self {
            items: DelayQueue::new(),
            sender,
            receiver,
        }
    }

    fn get_new(&mut self) {
        for _ in 0..10 {
            match self.receiver.poll() {
                Ok(Async::Ready(Some((duration, files)))) => {
                    let timestamp = Instant::now() + ((duration / 100) * 150);
                    debug!("{} files queued for cleanup at {:?}", files.len(), timestamp);
                    self.items.insert_at(files, timestamp);
                },
                _ => break,
            }
        }
    }

    pub fn sender(&self) -> Sender {
        self.sender.clone()
    }
}

impl Future for FileCleaner {
    type Item = ();
    type Error = ();

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        self.get_new();

        loop {
            match self.items.poll() {
                Ok(Async::NotReady) => return Ok(Async::NotReady),
                Ok(Async::Ready(None)) => return Ok(Async::NotReady),
                Ok(Async::Ready(Some(files))) => remove_files(&files.into_inner()),
                Err(why) => {
                    error!("{:?}", why);
                    return Err(());
                }
            }
        }
    }
}


fn remove_files(paths: &[PathBuf]) {
    debug!("Cleaning up {} files", paths.len());

    for path in paths {
        remove_file(path);
    }
}

fn remove_file(path: &PathBuf) {
    if let Err(why) = fs::remove_file(path) {
        error!("Failed to remove file '{}': {}", path.display(), why);
    }
}
