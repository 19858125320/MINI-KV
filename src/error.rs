use failure::Fail;
use std::{io,string::FromUtf8Error};
use sled;
/// Error type for kvs.
#[derive(Fail,Debug)]
pub enum KvsError {
    /// IO error.
    #[fail(display = "{}", _0)]
    Io(#[cause] io::Error),
    /// Serialization or deserialization error.
    #[fail(display = "{}", _0)]
    Serde(#[cause] serde_json::Error),

    /// bincode encode error.
    #[fail(display = "{}", _0)]
    BincodeEncodeError(#[cause] bincode::error::EncodeError),

    /// bincode decode error.
    #[fail(display = "{}", _0)]
    BincodeDecodeError(#[cause] bincode::error::DecodeError),
    /// Removing non-existent key error.
    #[fail(display = "Key not found")]
    KeyNotFound,
    /// the error about parse ip addr
    #[fail(display="{}",_0)]
    ParseIpError(#[cause] std::net::AddrParseError),
    #[fail(display = "decode command error")]
    DecodeError,
    /// Error with a string message
    #[fail(display = "{}", _0)]
    StringError(String),
    /// Sled error
    #[fail(display = "sled error: {}", _0)]
    Sled(#[cause] sled::Error),
    /// Key or value is invalid UTF-8 sequence
    #[fail(display = "UTF-8 error: {}", _0)]
    Utf8(#[cause] FromUtf8Error),
    /// Unexpected command type error.
    /// It indicated a corrupted log or a program bug.
    #[fail(display = "Unexpected command type")]
    UnexpectedCommandType,
    #[fail(display = "Invalid Command,must be [get <key>,scan <start> <end>,set <key> <value>,remove <key>]")]
    InvalidCommand,
}

impl From<io::Error> for KvsError {
    fn from(err: io::Error) -> KvsError {
        KvsError::Io(err)
    }
}

impl From<serde_json::Error> for KvsError {
    fn from(err: serde_json::Error) -> KvsError {
        KvsError::Serde(err)
    }
}

impl From<bincode::error::EncodeError> for KvsError {
    fn from(err: bincode::error::EncodeError) -> Self {
        KvsError::BincodeEncodeError(err)
    }
}

impl From<sled::Error> for KvsError {
    fn from(err: sled::Error) -> KvsError {
        KvsError::Sled(err)
    }
}

impl From<bincode::error::DecodeError> for KvsError {
    fn from(err: bincode::error::DecodeError) -> Self {
        KvsError::BincodeDecodeError(err)
    }
}

impl From<std::net::AddrParseError> for KvsError{
    fn from(err:std::net::AddrParseError)->Self{
        KvsError::ParseIpError(err)
    }
}

impl From<FromUtf8Error> for KvsError {
    fn from(err: FromUtf8Error) -> KvsError {
        KvsError::Utf8(err)
    }
}

/// Result type for kvs.
pub type Result<T> = std::result::Result<T, KvsError>;