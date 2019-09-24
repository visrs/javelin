#![warn(clippy::all)]

mod shared;
mod config;
mod media;
mod rtmp;
mod args;

#[cfg(feature = "hls")]
mod hls;

#[cfg(feature = "web")]
mod web;


use {
    futures::future::lazy,
    simplelog::{Config, SimpleLogger, TermLogger, LevelFilter},
};

#[allow(unused_imports)]
use self::shared::Shared;


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
        let config = shared.config.read().hls.clone();
        let hls_server = hls::Server::new(config);
        let hls_sender = hls_server.sender();
        shared.set_hls_sender(hls_sender);
        tokio::spawn(hls_server);
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
