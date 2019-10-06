use {
    std::fmt::Debug,
    futures::sync::{
        oneshot::Canceled,
        mpsc::SendError,
    },
    snafu::Snafu,
};
use super::{
    message::Message,
};


#[derive(Debug, Snafu)]
#[snafu(visibility(pub(super)))]
pub enum Error {
    BusSendFailed,
    BusResponseFailed { source: Canceled },
}
