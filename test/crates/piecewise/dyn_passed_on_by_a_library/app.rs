//! Writes a `dyn Display` of its own into a writer a piece at a time, beside
//! a library that only passes a `dyn` on: not refused (#1712).
use std::fmt::{self, Write};

struct Log(Vec<String>);

impl Write for Log {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.0.push(s.to_string());
        Ok(())
    }
}

struct Point(i32);

impl fmt::Display for Point {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({})", self.0)
    }
}

pub fn main() {
    let mut log = Log(Vec::new());
    let shown: &dyn fmt::Display = &Point(3);
    write!(log, "[{shown}]").unwrap();
    let err = "x".parse::<u8>().unwrap_err();
    let wrapped = dep::Wrapped(err);
    println!("{:?} {wrapped} {}", log.0, std::error::Error::source(&wrapped).is_none());
}
