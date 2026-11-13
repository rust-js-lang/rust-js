//! A library's generic writer, which writes what it's given into a writer
//! a piece at a time through its dictionary, and a type of its own it
//! writes so, whose dictionary its contract says carries `fmtInto`
//! (#1652, step 5 of ADR 1135).
use std::fmt::{self, Write};

pub fn framed<W: Write, T: fmt::Display>(w: &mut W, t: &T) -> fmt::Result {
    write!(w, "[{:>4}]", t)
}

pub struct Meters(pub u32);

impl fmt::Display for Meters {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}m", self.0)
    }
}

pub fn meters_into<W: Write>(w: &mut W, m: &Meters) -> fmt::Result {
    framed(w, m)
}
