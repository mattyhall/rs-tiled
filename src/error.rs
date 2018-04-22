use std;
use std::io::Error;
use std::fmt;
use xml::reader::Error as XmlError;
use base64::Base64Error;

/// Errors which occured when parsing the file
#[derive(Debug)]
pub enum TiledError {
    /// A attribute was missing, had the wrong type of wasn't formated
    /// correctly.
    MalformedAttributes(String),
    /// An error occured when decompressing using the
    /// [flate2](https://github.com/alexcrichton/flate2-rs) crate.
    DecompressingError(Error),
    Base64DecodingError(Base64Error),
    XmlDecodingError(XmlError),
    PrematureEnd(String),
    Other(String),
}

impl fmt::Display for TiledError {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match *self {
            TiledError::MalformedAttributes(ref s) => write!(fmt, "{}", s),
            TiledError::DecompressingError(ref e) => write!(fmt, "{}", e),
            TiledError::Base64DecodingError(ref e) => write!(fmt, "{}", e),
            TiledError::XmlDecodingError(ref e) => write!(fmt, "{}", e),
            TiledError::PrematureEnd(ref e) => write!(fmt, "{}", e),
            TiledError::Other(ref s) => write!(fmt, "{}", s),
        }
    }
}

// This is a skeleton implementation, which should probably be extended in the future.
impl std::error::Error for TiledError {
    fn description(&self) -> &str {
        match *self {
            TiledError::MalformedAttributes(ref s) => s.as_ref(),
            TiledError::DecompressingError(ref e) => e.description(),
            TiledError::Base64DecodingError(ref e) => e.description(),
            TiledError::XmlDecodingError(ref e) => e.description(),
            TiledError::PrematureEnd(ref s) => s.as_ref(),
            TiledError::Other(ref s) => s.as_ref(),
        }
    }
    fn cause(&self) -> Option<&std::error::Error> {
        match *self {
            TiledError::MalformedAttributes(_) => None,
            TiledError::DecompressingError(ref e) => Some(e as &std::error::Error),
            TiledError::Base64DecodingError(ref e) => Some(e as &std::error::Error),
            TiledError::XmlDecodingError(ref e) => Some(e as &std::error::Error),
            TiledError::PrematureEnd(_) => None,
            TiledError::Other(_) => None,
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub enum ParseTileError {
    ColourError,
    OrientationError,
}
