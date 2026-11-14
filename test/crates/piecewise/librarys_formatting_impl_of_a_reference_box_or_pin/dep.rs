//! A library's formatting impls of a reference to, a `Box` of and a
//! `Pin<Box<..>>` of its own types: each type is published as one a
//! formatting impl is of, so its contract lists it and a consumer writes it
//! a piece at a time (#1798, #1688).
use std::fmt;
use std::pin::Pin;

pub struct ByRef(pub u8);

impl fmt::Display for &ByRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "r{}", self.0)
    }
}

impl fmt::Debug for &ByRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ByRef({})", self.0)
    }
}

pub struct Boxed(pub u8);

impl fmt::Display for Box<Boxed> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "b{}", self.0)
    }
}

pub struct Pinned(pub u8);

impl fmt::Display for Pin<Box<Pinned>> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "p{}", self.0)
    }
}
