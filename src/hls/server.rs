use {
    std::{io::Result, fs, path::Path},
    log::{debug, error, info},
    tokio::prelude::*,
    futures::{try_ready, sync::{mpsc, oneshot}},
};
use crate::{
    media,
};
use super::{
    config::Config,
    writer::Writer,
    file_cleaner,
};


type Message = (String, oneshot::Sender<media::Sender>);
type Receiver = mpsc::UnboundedReceiver<Message>;
pub type Sender = mpsc::UnboundedSender<Message>;


enum State {
    Initializing,
    Listening(file_cleaner::Sender),
}


pub struct Server {
    state: State,
    config: Config,
    sender: Sender,
    receiver: Receiver,
}

impl Server {
    pub fn new(config: Config) -> Self {
        let (sender, receiver) = mpsc::unbounded();

        let hls_root = &config.root_dir;
        info!("HLS directory located at '{}'", hls_root.display());

        debug!("Attempting cleanup of HLS directory");
        directory_cleanup(hls_root).expect("Failed to clean up HLS directory");
        info!("HLS directory purged");

        Self {
            state: State::Initializing,
            config,
            sender,
            receiver
        }
    }

    pub fn spawn(config: Config) {
        if config.enabled {
            let server = Self::new(config); // .map_err(|e| error!("{}", e));
            tokio::spawn(server);
        }
    }

    pub fn sender(&self) -> Sender {
        self.sender.clone()
    }

    fn initialize(&mut self) {
        debug!("HLS: Intializing");
        let file_cleaner = file_cleaner::FileCleaner::new();
        let file_cleaner_sender = file_cleaner.sender();
        tokio::spawn(file_cleaner);
        self.state = State::Listening(file_cleaner_sender);
    }
}

impl Future for Server {
    type Item = ();
    type Error = ();

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        match self.state {
            State::Initializing => {
                self.initialize();
                // Notify runtime to poll this future again as soon as possible
                task::current().notify();
                Ok(Async::NotReady)
            },
            State::Listening(ref file_cleaner) => {
                debug!("HLS: Listening for requests");

                while let Some((app_name, request)) = try_ready!(self.receiver.poll()) {
                    let (sender, receiver) = mpsc::unbounded();
                    request.send(sender).unwrap();

                    let config = self.config.clone();

                    match Writer::create(app_name, receiver, config, file_cleaner.clone()) {
                        Ok(writer) => { tokio::spawn(writer); },
                        Err(why) => error!("Failed to create writer: {:?}", why),
                    }
                }

                Ok(Async::Ready(()))
            },
        }
    }
}


fn directory_cleanup<P: AsRef<Path>>(path: P) -> Result<()> {
    let path = path.as_ref();

    if path.exists() {
        if path.is_dir() {
            for entry in fs::read_dir(path)? {
                let child_path = entry?.path();
                if child_path.is_dir() {
                    fs::remove_dir_all(child_path)?;
                } else {
                    fs::remove_file(child_path)?;
                }
            }
        } else {
            panic!("HLS root is not a directory, aborting");
        }
    }

    Ok(())
}
