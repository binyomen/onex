use {
    std::{error, fmt, io, path::StripPrefixError},
    zip::result::ZipError,
};

#[derive(Debug)]
pub struct ErrorInternal {
    msg: String,
}

impl fmt::Display for ErrorInternal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.msg.fmt(f)
    }
}

impl error::Error for ErrorInternal {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        None
    }
}

#[derive(Debug)]
pub enum Error {
    Sexe(ErrorInternal),
    Io(io::Error),
    Zip(ZipError),
    Walkdir(walkdir::Error),
    StripPrefix(StripPrefixError),
    CtrlC(ctrlc::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Sexe(err) => err.fmt(f),
            Error::Io(err) => err.fmt(f),
            Error::Zip(err) => err.fmt(f),
            Error::Walkdir(err) => err.fmt(f),
            Error::StripPrefix(err) => err.fmt(f),
            Error::CtrlC(err) => err.fmt(f),
        }
    }
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            Error::Sexe(err) => Some(err),
            Error::Io(err) => Some(err),
            Error::Zip(err) => Some(err),
            Error::Walkdir(err) => Some(err),
            Error::StripPrefix(err) => Some(err),
            Error::CtrlC(err) => Some(err),
        }
    }
}

impl From<&str> for Error {
    fn from(msg: &str) -> Self {
        Error::Sexe(ErrorInternal {
            msg: msg.to_owned(),
        })
    }
}

impl From<ErrorInternal> for Error {
    fn from(err: ErrorInternal) -> Self {
        Error::Sexe(err)
    }
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Self {
        Error::Io(err)
    }
}

impl From<ZipError> for Error {
    fn from(err: ZipError) -> Self {
        Error::Zip(err)
    }
}

impl From<walkdir::Error> for Error {
    fn from(err: walkdir::Error) -> Self {
        Error::Walkdir(err)
    }
}

impl From<StripPrefixError> for Error {
    fn from(err: StripPrefixError) -> Self {
        Error::StripPrefix(err)
    }
}

impl From<ctrlc::Error> for Error {
    fn from(err: ctrlc::Error) -> Self {
        Error::CtrlC(err)
    }
}

pub type Result<T> = std::result::Result<T, Error>;
