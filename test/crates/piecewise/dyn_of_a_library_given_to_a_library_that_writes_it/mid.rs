//@ uses:
//! A library that writes a `dyn Debug` it's given into a writer a piece at
//! a time, as its contract says (`dyn_written_into`); `dep`'s sibling,
//! which it doesn't use.
use std::fmt::{self, Write};

pub fn show<W: Write>(w: &mut W, d: &dyn fmt::Debug) -> fmt::Result {
    write!(w, "[{d:?}]")
}
