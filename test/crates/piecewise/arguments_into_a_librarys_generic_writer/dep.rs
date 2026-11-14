//! A library's generic writer, which writes what it's given into a writer
//! a piece at a time through its dictionary (#1652, step 5 of ADR 1135).
use std::fmt::{self, Write};

pub fn framed<W: Write, T: fmt::Display>(w: &mut W, t: &T) -> fmt::Result {
    write!(w, "[{}]", t)
}
