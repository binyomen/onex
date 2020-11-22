use {
    macros::ErrorEnum,
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

#[derive(Debug, ErrorEnum)]
pub enum Error {
    Sexe(ErrorInternal),
    Io(io::Error),
    Zip(ZipError),
    Walkdir(walkdir::Error),
    StripPrefix(StripPrefixError),
}

impl From<&str> for Error {
    fn from(msg: &str) -> Self {
        ErrorInternal {
            msg: msg.to_owned(),
        }
        .into()
    }
}

pub type Result<T> = std::result::Result<T, Error>;
