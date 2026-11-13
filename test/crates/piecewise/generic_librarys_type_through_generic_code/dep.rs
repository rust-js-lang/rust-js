//! A library's generic type, which isn't published yet (#1712): its
//! dictionary carries no `fmtInto`, which its contract says.
use std::fmt;

pub struct Wrap<T>(pub T);

impl<T: fmt::Display> fmt::Display for Wrap<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "<{}>", self.0)
    }
}
