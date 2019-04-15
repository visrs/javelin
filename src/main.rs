#![warn(clippy::all)]
#![allow(dead_code)] // FIXME: remove this
#![allow(unused_imports)] // FIXME: remove this

mod error;
mod shared;
mod config;
mod media;
mod rtmp;

#[cfg(feature = "hls")]
mod hls;

#[cfg(feature = "web")]
mod web;


use futures::future::lazy;
use simplelog::{Config, SimpleLogger, TermLogger, LevelFilter};

#[allow(unused_imports)]
use self::{
    shared::Shared,
    error::{Error, Result},
};


macro_rules! init_logger {
    [ $kind:ident ] => { $kind::init(LevelFilter::Debug, Config::default()) }
}


fn main() {
    init_logger!(TermLogger).unwrap_or_else(|_|
        init_logger!(SimpleLogger).unwrap_or_else(|err|
            eprintln!("Failed to initialize logger: {}", err)));

    let shared = Shared::new();

    #[cfg(feature = "web")]
    spawn_web_server(shared.clone());

    tokio::run(lazy(move || {
        #[cfg(feature = "hls")]
        spawn_hls_server(shared.clone());

        tokio::spawn(rtmp::Server::new(shared.clone()));

        Ok(())
    }));
}

#[cfg(feature = "hls")]
fn spawn_hls_server(mut shared: Shared) {
    let enabled = {
        let config = shared.config.read();
        config.hls.enabled
    };

    if enabled {
        let hls_server = hls::Server::new(shared.clone());
        let hls_sender = hls_server.sender();
        let file_cleaner = hls::file_cleaner::FileCleaner::new(shared.clone());
        shared.set_hls_sender(hls_sender);
        tokio::spawn(hls_server);
        tokio::spawn(file_cleaner);
    }
}

#[cfg(feature = "web")]
fn spawn_web_server(shared: Shared) {
    let enabled = {
        let config = shared.config.read();
        config.hls.enabled && config.web.enabled
    };

    if enabled {
        web::Server::new(shared).start();
    }
}
