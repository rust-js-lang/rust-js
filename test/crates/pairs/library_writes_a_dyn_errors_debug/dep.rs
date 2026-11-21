//! A library's error holding a `dyn Error`, shown by its `Debug`, as
//! rand_core 0.5.1's `Error` writes `self.inner`: written into a writer
//! of its user's own, a piece at a time, as std writes each error (#2250).
use std::error::Error;
use std::fmt;

pub struct Wrapped {
    inner: Box<dyn Error + Send + Sync + 'static>,
}

impl Wrapped {
    pub fn new<E>(err: E) -> Self
    where
        E: Into<Box<dyn Error + Send + Sync + 'static>>,
    {
        Wrapped { inner: err.into() }
    }
}

impl fmt::Debug for Wrapped {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Wrapped {{ inner: {:?} }}", self.inner)
    }
}

impl fmt::Display for Wrapped {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.inner)
    }
}

/// The library's own error type, which it makes a `dyn Error` of.
#[derive(Debug)]
pub struct Code(pub u32);

impl fmt::Display for Code {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "code {}", self.0)
    }
}

impl Error for Code {}

pub fn coded(code: u32) -> Wrapped {
    Wrapped::new(Code(code))
}

pub fn parsed(text: &str) -> Wrapped {
    Wrapped::new(text.parse::<u8>().unwrap_err())
}
