//! A library whose writer shows its dependency's writer that may stop
//! part-way, by a panic: so may its own, as its contract says (#1608).
use std::fmt;

use dep::Loud;

pub struct Holder(pub Loud);

impl fmt::Debug for Holder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "<{:?}>", self.0)
    }
}
