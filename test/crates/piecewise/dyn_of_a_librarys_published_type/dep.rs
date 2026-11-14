//! A library that makes `dyn`s of published types of its own whose
//! formatting impls are written by hand: a `Box<dyn Display>` of `Meters`,
//! and `?`'s conversion of its `Bad` into a `Box<dyn Error>`. Their
//! dictionaries carry `fmtInto`, so its contract doesn't say it makes a
//! `dyn` that may carry none (#1688).
use std::error::Error;
use std::fmt;

pub struct Meters(pub u32);

impl fmt::Display for Meters {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}m", self.0)
    }
}

pub fn boxed(n: u32) -> Box<dyn fmt::Display> {
    Box::new(Meters(n))
}

pub struct Bad(pub u8);

impl fmt::Display for Bad {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "bad {}", self.0)
    }
}

impl fmt::Debug for Bad {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Bad({})", self.0)
    }
}

impl Error for Bad {}

fn check(n: u8) -> Result<u8, Bad> {
    if n > 9 { Err(Bad(n)) } else { Ok(n) }
}

pub fn parse(n: u8) -> Result<u8, Box<dyn Error>> {
    Ok(check(n)?)
}
