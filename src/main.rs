#![warn(clippy::all)]

#![allow(unused_variables)]
#![allow(unused_imports)]

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
    simplelog::{
        Config as LogConfig,
        SimpleLogger, TermLogger, LevelFilter
    },
    javelin_core::session::{self, SessionBusSender},
};

use crate::{
    config::Config,
    shared::Shared,
};


macro_rules! init_logger {
    [ $kind:ident ] => { $kind::init(LevelFilter::Debug, LogConfig::default()) }
}


fn main() {
    init_logger!(TermLogger).unwrap_or_else(|_|
        init_logger!(SimpleLogger).unwrap_or_else(|err|
            eprintln!("Failed to initialize logger: {}", err)));

    let config = Config::new();
    let shared = Shared::new();

    #[cfg(feature = "web")]
    web::Server::spawn(config.clone(), shared.clone());

    tokio::run(lazy(move || {
        let allowed_sessions = shared.config.read().rtmp.permitted_stream_keys.clone();
        let session_manager = session::SessionManager::new(allowed_sessions);
        let session_sender = session_manager.sender();
        tokio::spawn(session_manager);

        #[cfg(feature = "hls")]
        hls::Server::spawn(config.hls.clone());

        spawn_rtmp_server(shared.clone());

        Ok(())
    }));
}

fn spawn_rtmp_server(shared: Shared) {
    let config = shared.config.read().rtmp.clone();
    tokio::spawn(rtmp::Server::new(config, shared.clone()));
}
