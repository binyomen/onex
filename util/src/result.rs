use {
    std::{error, fmt, io, path::StripPrefixError},
    zip::result::ZipError,
};

#[derive(Debug)]
pub struct SexeErrorInternal {
    msg: String,
}

impl fmt::Display for SexeErrorInternal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.msg.fmt(f)
    }
}

impl error::Error for SexeErrorInternal {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        None
    }
}

#[derive(Debug)]
pub enum SexeError {
    Sexe(SexeErrorInternal),
    Io(io::Error),
    Zip(ZipError),
    Walkdir(walkdir::Error),
    StripPrefix(StripPrefixError),
    CtrlC(ctrlc::Error),
}

impl fmt::Display for SexeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SexeError::Sexe(err) => err.fmt(f),
            SexeError::Io(err) => err.fmt(f),
            SexeError::Zip(err) => err.fmt(f),
            SexeError::Walkdir(err) => err.fmt(f),
            SexeError::StripPrefix(err) => err.fmt(f),
            SexeError::CtrlC(err) => err.fmt(f),
        }
    }
}

impl error::Error for SexeError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            SexeError::Sexe(err) => Some(err),
            SexeError::Io(err) => Some(err),
            SexeError::Zip(err) => Some(err),
            SexeError::Walkdir(err) => Some(err),
            SexeError::StripPrefix(err) => Some(err),
            SexeError::CtrlC(err) => Some(err),
        }
    }
}

impl From<&str> for SexeError {
    fn from(msg: &str) -> Self {
        SexeError::Sexe(SexeErrorInternal {
            msg: msg.to_owned(),
        })
    }
}

impl From<SexeErrorInternal> for SexeError {
    fn from(err: SexeErrorInternal) -> Self {
        SexeError::Sexe(err)
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

impl From<ctrlc::Error> for SexeError {
    fn from(err: ctrlc::Error) -> Self {
        SexeError::CtrlC(err)
    }
}

pub type SexeResult<T> = Result<T, SexeError>;
