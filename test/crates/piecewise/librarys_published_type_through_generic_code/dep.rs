//! A library's type reachable from outside it, which the library never
//! writes into a writer itself: as a consumer may, its writer writes
//! through a sink and its dictionary carries `fmtInto`, which its contract
//! lists (`written_into`, ADR 1135's switch-over). One of a lifetime alone
//! is published too, as a lifetime changes nothing of its JS; and one whose
//! `Debug` is derived, which the library exports none of and a consumer
//! compiles its own of, framed as the library's would be.
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
