//! A library's formatting impls of a `&mut` of its own type, and of a `&`:
//! a consumer writes it a piece at a time by the dictionary the library's
//! contract lists, given the `&mut` as that dictionary takes one (#1824).
use std::fmt;

pub struct ByRef(pub u8);

impl fmt::Display for &mut ByRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "m{}", self.0)
    }
}

impl fmt::Debug for &ByRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "R{}", self.0)
    }
}
