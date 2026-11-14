//! A library whose `?` converts its own error into a `Box<dyn Error>`,
//! std's `From`: it makes a `dyn` its contract says may carry no `fmtInto`
//! (`makes_fmt_dyns`, #1712).
use std::error::Error;
use std::fmt;

#[derive(Debug)]
struct Bad(u8);

impl fmt::Display for Bad {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "bad {}", self.0)
    }
}

impl Error for Bad {}

fn check(n: u8) -> Result<u8, Bad> {
    if n > 9 { Err(Bad(n)) } else { Ok(n) }
}

pub fn parse(n: u8) -> Result<u8, Box<dyn Error>> {
    Ok(check(n)?)
}
