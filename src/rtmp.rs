mod event;
mod error;
pub mod peer;
pub mod client;
pub mod server;


pub type ClientId = usize;


use self::{
    peer::Peer,
};

pub use self::{
    client::Client,
    server::Server,
};
