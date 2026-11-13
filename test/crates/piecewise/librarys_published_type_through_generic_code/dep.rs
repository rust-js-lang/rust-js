//! A library's type reachable from outside it, which the library never
//! writes into a writer itself: as a consumer may, its writer writes
//! through a sink and its dictionary carries `fmtInto`, which its contract
//! lists (`written_into`, ADR 1135's switch-over).
use std::fmt;

pub struct Meters(pub u32);

impl fmt::Display for Meters {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}m", self.0)
    }
}
