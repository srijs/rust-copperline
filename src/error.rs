use std::error;
use std::fmt;
use nix;

#[derive(Debug, PartialEq)]
pub enum Error {
    ErrNo(nix::Error),
    Cancel,
    EndOfFile,
    UnsupportedTerm,
    ParseError
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::ErrNo(ref err)  => write!(f, "ERRNO: {}", err.errno().desc()),
            Error::Cancel          => write!(f, "Cancelled"),
            Error::EndOfFile       => write!(f, "End of file"),
            Error::UnsupportedTerm => write!(f, "Unsupported terminal type"),
            Error::ParseError      => write!(f, "Encountered unknown sequence")
        }
    }
}

impl error::Error for Error {
    fn description(&self) -> &str {
        match *self {
            Error::ErrNo(ref err)  => err.errno().desc(),
            Error::Cancel          => "cancelled",
            Error::EndOfFile       => "end of file",
            Error::UnsupportedTerm => "unsupported terminal type",
            Error::ParseError      => "unknown sequence"
        }
    }
}

impl From<nix::Error> for Error {
    fn from(err: nix::Error) -> Error {
        Error::ErrNo(err)
    }
}
