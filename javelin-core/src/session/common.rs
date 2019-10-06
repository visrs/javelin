use {
    futures::sync::mpsc,
    bytes::Bytes,
};


pub type DataSender = mpsc::UnboundedSender<Bytes>;
pub type DataReceiver = mpsc::UnboundedReceiver<Bytes>;


pub fn data_channel() -> (DataSender, DataReceiver) {
    mpsc::unbounded()
}
