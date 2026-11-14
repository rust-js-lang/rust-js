//! A library's writer functions another crate can reach, framed as each
//! may be given a consumer's `Formatter` or named a `fn` pointer a framed
//! writer calls, a generic one too (#1760): `Via` calls what it holds.
use std::fmt;

pub trait Is<T> {
    fn is(self) -> T;
}

impl<T> Is<T> for T {
    fn is(self) -> T {
        self
    }
}

pub type Writing = fn(&mut fmt::Formatter<'_>, i32) -> fmt::Result;

pub fn bracketed(f: &mut fmt::Formatter<'_>, n: impl Is<i32>) -> fmt::Result {
    let n: i32 = Is::is(n);
    write!(f, "[{n}]")
}

pub fn plain(f: &mut fmt::Formatter<'_>, n: i32) -> fmt::Result {
    write!(f, "<{n}>")
}

pub struct Via(pub Writing, pub i32);

impl fmt::Display for Via {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        (self.0)(f, self.1)
    }
}
