use {
    std::{error, fmt, io, path::StripPrefixError},
    zip::result::ZipError,
};

#[derive(Debug)]
pub enum SexeError {
    Io(io::Error),
    Zip(ZipError),
    Walkdir(walkdir::Error),
    StripPrefix(StripPrefixError),
}

impl fmt::Display for SexeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SexeError::Io(err) => err.fmt(f),
            SexeError::Zip(err) => err.fmt(f),
            SexeError::Walkdir(err) => err.fmt(f),
            SexeError::StripPrefix(err) => err.fmt(f),
        }
    }
}

impl error::Error for SexeError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            SexeError::Io(err) => Some(err),
            SexeError::Zip(err) => Some(err),
            SexeError::Walkdir(err) => Some(err),
            SexeError::StripPrefix(err) => Some(err),
        }
    }
}

impl From<io::Error> for SexeError {
    fn from(err: io::Error) -> Self {
        SexeError::Io(err)
    }
}

impl From<ZipError> for SexeError {
    fn from(err: ZipError) -> Self {
        SexeError::Zip(err)
    }
}

impl From<walkdir::Error> for SexeError {
    fn from(err: walkdir::Error) -> Self {
        SexeError::Walkdir(err)
    }
}

impl From<StripPrefixError> for SexeError {
    fn from(err: StripPrefixError) -> Self {
        SexeError::StripPrefix(err)
    }
}

pub type SexeResult<T> = Result<T, SexeError>;
