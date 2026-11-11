//! A library that makes a `Box<dyn Error>` of its error, which has a
//! destructor, and hands it to its consumer: its contract says so,
//! `makes_dropping_errors`, so the consumer drops it (#1403, ADR 0462).
use std::error::Error;
use std::fmt;

#[derive(Debug)]
pub struct Loud(pub u32);

impl fmt::Display for Loud {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "loud {}", self.0)
    }
}

impl Error for Loud {}

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop loud {}", self.0);
    }
}

pub fn failing(n: u32) -> Result<u32, Box<dyn Error>> {
    if n > 1 {
        return Err(Box::new(Loud(n)));
    }
    Ok(n)
}
