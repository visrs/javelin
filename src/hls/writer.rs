use {
    std::{io, path::PathBuf, fs},
    log::{debug, error, warn},
    futures::try_ready,
    tokio::prelude::*,
    bytes::Bytes,
    chrono::Utc,
    snafu::{Snafu, ResultExt, ensure},
};
use javelin_codec::{self, avc, aac};
use crate::{
    media::{self, Media},
    config,
};
use super::{
    transport_stream::{
        self,
        Buffer as TsBuffer
    },
    m3u8::Playlist,
    file_cleaner,
};


#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("Path '{}' exists, but is not a directory", path.display()))]
    InvalidHlsRoot { path: PathBuf },

    #[snafu(display("Failed to create HLS root directory at '{}'", path.display()))]
    DirectoryCreationFailed { source: io::Error, path: PathBuf },

    #[snafu(display("Codec: {}", source))]
    CodecError { source: javelin_codec::Error },

    #[snafu(display("{}", source))]
    WriteError { source: transport_stream::Error },
}

type Result<T, E = Error> = std::result::Result<T, E>;


pub struct Writer {
    receiver: media::Receiver,
    write_interval: u64,
    next_write: u64,
    last_keyframe: u64,
    keyframe_counter: usize,
    buffer: TsBuffer,
    shared_state: javelin_codec::SharedState,
    playlist: Playlist,
    stream_path: PathBuf,
}

impl Writer {
    pub fn create(app_name: String, receiver: media::Receiver, config: config::HlsConfig, file_cleaner: file_cleaner::Sender) -> Result<Self> {
        let write_interval = 2000; // milliseconds
        let next_write = write_interval; // milliseconds

        let hls_root = config.root_dir;
        let stream_path = hls_root.join(app_name);
        let playlist_path = stream_path.join("playlist.m3u8");

        if stream_path.exists() {
            ensure!(stream_path.is_dir(), InvalidHlsRoot { path: stream_path.clone() });
        } else {
            debug!("Creating HLS directory at '{}'", stream_path.display());
            fs::create_dir_all(&stream_path).context(DirectoryCreationFailed { path: stream_path.clone() })?;
        }

        Ok(Self {
            receiver,
            write_interval,
            next_write,
            last_keyframe: 0,
            keyframe_counter: 0,
            buffer: TsBuffer::new(),
            shared_state: javelin_codec::SharedState::new(),
            playlist: Playlist::new(playlist_path, file_cleaner),
            stream_path,
        })
    }

    fn handle_h264<T>(&mut self, timestamp: T, bytes: Bytes) -> Result<()>
        where T: Into<u64>
    {
        let timestamp: u64 = timestamp.into();

        let packet = avc::Packet::try_from_buf(bytes, timestamp, &self.shared_state).context(CodecError)?;

        if packet.is_sequence_header() {
            return Ok(());
        }

        if packet.is_keyframe() {
            let keyframe_duration = timestamp - self.last_keyframe;

            if self.keyframe_counter == 1 {
                self.playlist.set_target_duration(keyframe_duration * 3);
            }

            if timestamp >= self.next_write {
                let filename = format!("{}-{}.ts", Utc::now().timestamp(), self.keyframe_counter);
                let path = self.stream_path.join(&filename);
                self.buffer.write_to_file(&path).context(WriteError)?;
                self.playlist.add_media_segment(filename, keyframe_duration);
                self.next_write += self.write_interval;
            }

            self.keyframe_counter += 1;
            self.last_keyframe = timestamp;
        }

        if let Err(why) = self.buffer.push_video(&packet) {
            warn!("Failed to put data into buffer: {:?}", why);
        }

        Ok(())
    }

    fn handle_aac<T>(&mut self, timestamp: T, bytes: Bytes) -> Result<()>
        where T: Into<u64>
    {
        let timestamp: u64 = timestamp.into();

        let packet = aac::Packet::try_from_bytes(bytes, timestamp, &self.shared_state).context(CodecError)?;

        if self.keyframe_counter == 0 || packet.is_sequence_header() {
            return Ok(());
        }

        if let Err(why) = self.buffer.push_audio(&packet) {
            warn!("Failed to put data into buffer: {:?}", why);
        }

        Ok(())
    }

    fn handle(&mut self, media: Media) -> Result<()> {
        match media {
            Media::H264(timestamp, bytes) => self.handle_h264(timestamp.value, bytes),
            Media::AAC(timestamp, bytes) => self.handle_aac(timestamp.value, bytes),
        }
    }
}

impl Future for Writer {
    type Item = ();
    type Error = ();

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        while let Some(media) = try_ready!(self.receiver.poll()) {
            self.handle(media).map_err(|why| error!("{:?}", why))?;
        }

        Ok(Async::Ready(()))
    }
}
