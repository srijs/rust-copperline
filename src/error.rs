use std::error;
use std::fmt;
use nix;

#[derive(Debug)]
pub enum Error {
    ErrNo(nix::Error),
    InvalidEncoding,
    EndOfFile,
    UnsupportedTerm
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::ErrNo(ref err)  => write!(f, "ERRNO: {}", err.errno().desc()),
            Error::InvalidEncoding => write!(f, "Invalid byte sequence"),
            Error::EndOfFile       => write!(f, "End of file"),
            Error::UnsupportedTerm => write!(f, "Unsupported terminal type")
        }
    }
}

impl error::Error for Error {
    fn description(&self) -> &str {
        match *self {
            Error::ErrNo(ref err)  => err.errno().desc(),
            Error::InvalidEncoding => "invalid byte sequence",
            Error::EndOfFile       => "end of file",
            Error::UnsupportedTerm => "unsupported terminal type"
        }
    }
}

impl From<nix::Error> for Error {
    fn from(err: nix::Error) -> Error {
        Error::ErrNo(err)
    }
}
