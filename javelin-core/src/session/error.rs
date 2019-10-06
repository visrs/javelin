use {
    snafu::Snafu,
};
use crate::{
    bus,
};


#[derive(Debug, Snafu)]
#[snafu(visibility(pub(super)))]
pub enum Error {
    BusError { source: bus::Error }
}
