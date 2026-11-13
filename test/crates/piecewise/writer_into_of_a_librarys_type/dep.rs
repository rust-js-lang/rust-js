//! A library's type, which the library never writes into a writer a piece
//! at a time, so its dictionary carries no `fmtInto` (#1652, #1688).
use std::fmt;

pub struct Meters(pub u32);

impl fmt::Display for Meters {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}m", self.0)
    }
}
