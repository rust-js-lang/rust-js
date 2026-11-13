//! A library's writer that may stop part-way, by a panic, after it wrote
//! part of its text (#1608, #1530).
use std::fmt;

pub struct Loud(pub u32);

impl fmt::Debug for Loud {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "!")?;
        if self.0 == 3 {
            panic!("stopped");
        }
        write!(f, "L{}", self.0)
    }
}
