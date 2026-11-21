//! A library's error made of an `io::Error`, rand 0.7.3's `ReadError`:
//! its derived `Debug` and its `Display`'s `write!` of the `io::Error`,
//! written into a writer of its user's own through rand_core's `Error`
//! a piece at a time, as std writes each (#1420).
use core::fmt;
pub struct Error {
    inner: Box<dyn std::error::Error + Send + Sync + 'static>,
}
impl Error {
    pub fn new<E>(err: E) -> Self
    where
        E: Into<Box<dyn std::error::Error + Send + Sync + 'static>>,
    {
        Error { inner: err.into() }
    }
}
impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Error {{ inner: {:?} }}", self.inner)
    }
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.inner)
    }
}

#[derive(Debug)]
struct ReadError(std::io::Error);
impl fmt::Display for ReadError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "error reading from Read source: {}", self.0)
    }
}
impl std::error::Error for ReadError {}

pub fn read_failed(e: std::io::Error) -> Error {
    Error::new(ReadError(e))
}
