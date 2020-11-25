use {
    macros::ErrorEnum,
    std::{error, fmt, io, path::StripPrefixError, sync::PoisonError},
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
pub struct PoisonErrorInternal(String);
impl fmt::Display for PoisonErrorInternal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}
impl error::Error for PoisonErrorInternal {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        None
    }
}

#[derive(Debug, ErrorEnum)]
pub enum Error {
    Sexe(ErrorInternal),
    Io(io::Error),
    Zip(ZipError),
    Walkdir(walkdir::Error),
    StripPrefix(StripPrefixError),
    Poison(PoisonErrorInternal),
}

impl From<&str> for Error {
    fn from(msg: &str) -> Self {
        ErrorInternal {
            msg: msg.to_owned(),
        }
        .into()
    }
}

impl<T> From<PoisonError<T>> for Error {
    fn from(err: PoisonError<T>) -> Self {
        Error::Poison(PoisonErrorInternal(format!("{}", err)))
    }
}

pub type Result<T> = std::result::Result<T, Error>;
