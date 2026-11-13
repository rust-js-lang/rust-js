//! A library's type reachable from outside it, which the library never
//! writes into a writer itself: as a consumer may, its writer writes
//! through a sink and its dictionary carries `fmtInto`, which its contract
//! lists (`written_into`, ADR 1135's switch-over). One of a lifetime alone
//! is published too, as a lifetime changes nothing of its JS; and one whose
//! `Debug` is derived, which the library exports none of and a consumer
//! compiles its own of, framed as the library's would be. A private type
//! only a published one's field holds, whose `Debug` nothing shows, as
//! rustls-pki-types' `BytesInner`, carries no `fmtInto` and isn't refused.
use std::fmt;

pub struct Meters(pub u32);

impl fmt::Display for Meters {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}m", self.0)
    }
}

pub struct Prefixed<'a>(pub &'a str, pub i32);

impl fmt::Display for Prefixed<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{:>3}", self.0, self.1)
    }
}

#[derive(Debug)]
pub struct Pair(pub i32, pub &'static str);

#[derive(Debug, Clone)]
enum Bytes<'a> {
    Owned(Vec<u8>),
    Borrowed(&'a [u8]),
}

#[derive(Clone)]
pub struct Der<'a>(Bytes<'a>);

pub fn der(owned: bool) -> Der<'static> {
    match owned {
        true => Der(Bytes::Owned(vec![1, 2, 3])),
        false => Der(Bytes::Borrowed(b"ab")),
    }
}

impl fmt::Debug for Der<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let len = match &self.0 {
            Bytes::Owned(bytes) => bytes.len(),
            Bytes::Borrowed(bytes) => bytes.len(),
        };
        write!(f, "Der({len})")
    }
}

#[derive(Clone, Debug)]
pub struct Anchor<'a> {
    pub subject: Der<'a>,
    pub constraints: Option<Der<'a>>,
}

/// A list a writer of a consumer's writes by the library's writer function,
/// which no writer of the library's own calls.
pub struct Listed(pub Vec<u32>);

impl Listed {
    pub fn joined(&self, f: &mut fmt::Formatter<'_>, sep: &str) -> fmt::Result {
        for (i, x) in self.0.iter().enumerate() {
            if i > 0 {
                f.write_str(sep)?;
            }
            write!(f, "{x}")?;
        }
        Ok(())
    }
}
