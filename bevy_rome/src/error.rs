#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Error {
    Unknown,
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Error::Unknown // TODO
    }
}

impl From<ron::Error> for Error {
    fn from(err: ron::Error) -> Self {
        Error::Unknown // TODO
    }
}

impl From<ron::error::SpannedError> for Error {
    fn from(err: ron::error::SpannedError) -> Self {
        Error::Unknown // TODO
    }
}
