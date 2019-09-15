use std::fmt::Debug;
use log::debug;
use bytes::Bytes;
use super::{
    error::Result,
    message,
};


pub(super) trait Event: Debug {}


pub(super) trait EventHandler<E>
    where E: Event
{
    fn handle_event(&mut self, event: E) -> Result<()>;
}


/// RTMP event types
pub(super) mod types {
    use super::{
        Event,
        message::{
            RequestId, StreamId, Timestamp, Metadata,
            VideoData, AudioData, ApplicationName, StreamKey,
        },
    };


    #[derive(Debug)]
    pub struct ConnectionRequested {
        pub id: RequestId,
        pub app_name: ApplicationName,
    }

    impl Event for ConnectionRequested {}


    #[derive(Debug)]
    pub struct PublishingRequested {
        pub id: RequestId,
        pub app_name: ApplicationName,
        pub stream_key: StreamKey,
    }

    impl Event for PublishingRequested {}


    #[derive(Debug)]
    pub struct PublishingFinished {
        pub app_name: ApplicationName,
        pub stream_key: StreamKey,
    }

    impl Event for PublishingFinished {}


    #[derive(Debug)]
    pub struct PlaybackRequested {
        pub id: RequestId,
        pub app_name: ApplicationName,
        pub stream_id: StreamId,
    }

    impl Event for PlaybackRequested {}


    #[derive(Debug)]
    pub struct PlaybackFinished {
        pub app_name: ApplicationName,
        pub stream_key: StreamKey,
    }

    impl Event for PlaybackFinished {}


    #[derive(Debug)]
    pub struct MetadataReceived {
        pub app_name: ApplicationName,
        pub metadata: Metadata,
    }

    impl Event for MetadataReceived {}


    #[derive(Debug)]
    pub struct VideoDataReceived {
        pub app_name: ApplicationName,
        pub stream_key: StreamKey,
        pub timestamp: Timestamp,
        pub payload: VideoData,
    }

    impl Event for VideoDataReceived {}


    #[derive(Debug)]
    pub struct AudioDataReceived {
        pub app_name: ApplicationName,
        pub stream_key: StreamKey,
        pub timestamp: Timestamp,
        pub payload: AudioData,
    }

    impl Event for AudioDataReceived {}


    #[derive(Debug)]
    pub struct SendMetadata {
        pub stream_id: StreamId,
        pub metadata: Metadata,
    }

    impl Event for SendMetadata {}


    #[derive(Debug)]
    pub struct SendVideoData {
        pub stream_id: StreamId,
        pub payload: VideoData,
        pub timestamp: Timestamp,
    }

    impl Event for SendVideoData {}


    #[derive(Debug)]
    pub struct SendAudioData {
        pub stream_id: StreamId,
        pub payload: AudioData,
        pub timestamp: Timestamp,
    }

    impl Event for SendAudioData {}
}
