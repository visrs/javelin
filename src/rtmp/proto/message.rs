use bytes::Bytes;
use rml_rtmp::{
    chunk_io,
    sessions::StreamMetadata,
    time::RtmpTimestamp,
};


pub type ApplicationName = String;
pub type StreamKey = String;
pub type RequestId = u32;
pub type StreamId = u32;
pub type Timestamp = RtmpTimestamp;
pub type Metadata = StreamMetadata;
pub type VideoData = Bytes;
pub type AudioData = Bytes;


#[derive(Debug, Clone)]
pub enum Message {
    Packet { droppable: bool, payload: Bytes },
    RegisterSource(ApplicationName),
    RegisterSink(ApplicationName),
    VideoData(Bytes, Timestamp),
    AudioData(Bytes, Timestamp),
    Metadata(Metadata),
    Finished,
}

impl From<&[u8]> for Message {
    fn from(val: &[u8]) -> Self {
        Self::Packet {
            droppable: false,
            payload: Bytes::from(val),
        }
    }
}

impl From<Vec<u8>> for Message {
    fn from(val: Vec<u8>) -> Self {
        Self::from(val.as_ref())
    }
}

impl From<chunk_io::Packet> for Message {
    fn from(val: chunk_io::Packet) -> Self {
        Self::Packet {
            droppable: val.can_be_dropped,
            payload: Bytes::from(val.bytes),
        }
    }
}

