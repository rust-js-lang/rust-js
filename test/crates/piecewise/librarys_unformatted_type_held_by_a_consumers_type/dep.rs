//! A library's type no formatting impl is of, which has no formatting
//! dictionary, and a generic type whose `Display` never writes what it
//! holds: neither is listed among the types whose dictionaries carry
//! `fmtInto`, nor need be (#1688).
use std::fmt;

pub struct Token(pub u8);

pub struct Tag<T>(pub T);

impl<T> fmt::Display for Tag<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("tag")
    }
}

pub struct Shown(pub i32);

impl fmt::Display for Shown {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "<{}>", self.0)
    }
}
