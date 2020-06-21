use serde::{de, ser};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("{0}")]
    Message(String),
    // TODO: cannot be shared between threads ... other solution?
    // #[error("Lua error: {0}")]
    // Lua(#[from] mlua::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

impl ser::Error for Error {
    fn custom<T: std::fmt::Display>(msg: T) -> Self {
        Error::Message(msg.to_string())
    }
}

impl de::Error for Error {
    fn custom<T: std::fmt::Display>(msg: T) -> Self {
        Error::Message(msg.to_string())
    }
}

impl From<mlua::Error> for Error {
    fn from(err: mlua::Error) -> Self {
        Error::Message(err.to_string())
    }
}
