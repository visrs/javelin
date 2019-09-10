use std::{io, result, error, fmt};


pub type Result<T> = result::Result<T, Error>;


#[derive(Debug)]
pub enum Error {
    IoError(io::Error),
    Custom(String),
    ParseError(String),
    NotEnoughData,
    DecoderConfigurationRecordMissing,
    AudioSpecificConfigurationMissing,
    UnsupportedConfigurationRecordVersion(u8),
}

impl error::Error for Error {}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut w = |s: &str| { write!(f, "{}", s) };

        match self {
            Error::IoError(inner) => w(&inner.to_string()),
            Error::Custom(inner) => w(inner),
            Error::ParseError(inner) => w(inner),
            Error::NotEnoughData => w("Not enough data"),
            Error::DecoderConfigurationRecordMissing => w("Decoder configuration record missing"),
            Error::AudioSpecificConfigurationMissing => w("Audio specific configuration missing"),
            Error::UnsupportedConfigurationRecordVersion(ver) => w(&format!("Unsupported configuration record version {}", ver)),
        }
    }
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Self {
        Error::IoError(err)
    }
}

impl<'a> From<&'a str> for Error {
    fn from(err: &'a str) -> Self {
        Error::Custom(err.to_string())
    }
}

impl From<String> for Error {
    fn from(err: String) -> Self {
        Error::Custom(err)
    }
}
