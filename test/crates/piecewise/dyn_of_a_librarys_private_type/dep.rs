//! A library that makes a `dyn Display` of a private type of its own,
//! whose `Display` is written by hand but isn't published: its writer isn't
//! framed and its dictionary carries no `fmtInto`, so its contract says it
//! makes a `dyn` that may carry none (#1797).
use std::fmt;

struct Inches(u32);

impl fmt::Display for Inches {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}in", self.0)
    }
}

pub fn boxed(n: u32) -> Box<dyn fmt::Display> {
    Box::new(Inches(n))
}
