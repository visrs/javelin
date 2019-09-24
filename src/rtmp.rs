mod bytes_stream;
mod event;
mod error;
pub mod peer;
pub mod client;
pub mod server;


pub type ClientId = usize;


use self::{
    peer::Peer,
    bytes_stream::BytesStream,
};

pub use self::{
    client::Client,
    server::Server,
};
