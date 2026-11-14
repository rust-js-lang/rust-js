//! A library that passes a `dyn Error` on, as either's `Error` does,
//! `self.0.source()`, or one a coercion of another `dyn` gives, `dyn Error
//! + Send + Sync` as a `dyn Error`, and makes none of its own: its contract
//! doesn't say it makes a `dyn` whose dictionary may carry no `fmtInto`
//! (`makes_fmt_dyns`, #1712).
use std::error::Error;
use std::fmt;

#[derive(Debug)]
pub struct Wrapped<E>(pub E);

impl<E: fmt::Display> fmt::Display for Wrapped<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "wrapped {}", self.0)
    }
}

impl<E: Error> Error for Wrapped<E> {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.0.source()
    }
}

#[derive(Debug)]
pub struct Chained {
    pub inner: Option<Box<dyn Error + Send + Sync>>,
}

impl fmt::Display for Chained {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("chained")
    }
}

impl Error for Chained {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.inner.as_deref().map(|e| e as &(dyn Error + 'static))
    }
}
