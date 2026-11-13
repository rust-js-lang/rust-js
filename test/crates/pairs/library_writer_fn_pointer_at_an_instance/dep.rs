//! A library's generic writer function, which a consumer names as a `fn`
//! pointer at one of its instances (#1710).
use std::fmt;

pub trait Is<T> {
    fn is(self) -> T;
}

impl<T> Is<T> for T {
    fn is(self) -> T {
        self
    }
}

pub fn bracketed(f: &mut fmt::Formatter<'_>, n: impl Is<i32>) -> fmt::Result {
    let n: i32 = Is::is(n);
    write!(f, "[{n}]")
}
