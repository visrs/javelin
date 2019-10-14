use std::{
    error::Error as StdError,
    thread,
};
use warp::{
    Filter,
    Reply,
    Rejection,
    http::StatusCode,
};
use serde_json::json;
use super::api::{
    api,
    Error as ApiError,
};
use crate::{
    config::Config,
    Shared
};


macro_rules! json_error_response {
    ($code:expr, $message:expr) => {{
        let json = json!({ "error": $message });
        let reply = warp::reply::json(&json);
        Ok(warp::reply::with_status(reply, $code))
    }};
}


pub struct Server {
    config: Config,
}

impl Server {
    pub fn new(config: Config) -> Self {
        Self { config }
    }

    pub fn spawn(config: Config, shared: Shared) {
        if config.web.enabled {
            let server = Self::new(config);
            thread::spawn(|| start(server, shared));
        }
    }
}


fn start(server: Server, shared: Shared) {
    let config = server.config;

    let routes = warp::path("api")
        .and(api(shared.clone()));

    #[cfg(feature = "hls")]
    let routes = routes
        .or(warp::path("hls")
            .and(warp::fs::dir(config.hls.root_dir)));

    let routes = routes
        .recover(error_handler);

    warp::serve(routes).run(config.web.addr);
}

fn error_handler(err: Rejection) -> Result<impl Reply, Rejection> {
    match err.find_cause() {
        | Some(e @ ApiError::NoSuchResource)
        | Some(e @ ApiError::StreamNotFound) => {
            json_error_response!(StatusCode::NOT_FOUND, e.description())
        },
        None => Err(err)
    }
}
