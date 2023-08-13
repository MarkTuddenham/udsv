use std;
use std::fmt::{self, Display};

use serde::{de, ser};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    Message(String),

    Eof,
    Syntax,
    BytesUnsupported,
    IntegerOverflow,
    ExpectedBoolean,
    ExpectedInteger,
    ExpectedChar,
    ExpectedString,
    ExpectedEmpty,
    ExpectedArray,
    ExpectedArrayComma,
    ExpectedArrayEnd,
    ExpectedMap,
    ExpectedMapComma,
    ExpectedMapEquals,
    ExpectedMapEnd,
    ExpectedEnum,
    TrailingCharacters,
}

impl ser::Error for Error {
    fn custom<T: Display>(msg: T) -> Self {
        Error::Message(msg.to_string())
    }
}

impl de::Error for Error {
    fn custom<T: Display>(msg: T) -> Self {
        Error::Message(msg.to_string())
    }
}

impl Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::Message(msg) => formatter.write_str(msg),
            Error::Eof => formatter.write_str("Unexpected end of input"),
            Error::BytesUnsupported => formatter
                .write_str("Serialising bytes is not supported for a human readable format"),
            _ => formatter.write_str("I haven't implemented this error message yet"),
            // TODO: Implement the rest of the error messages
            /* and so forth */
        }
    }
}

impl std::error::Error for Error {}
