//! Gives the library's generic writer a type of its own, which it writes
//! into nothing itself: its writer writes a piece at a time all the same,
//! as the library's contract asks of what its parameter's given (#1652).
use std::fmt::{self, Write};

struct Log(Vec<String>);

impl Write for Log {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.0.push(s.to_string());
        Ok(())
    }
}

struct Feet(i32);

impl fmt::Display for Feet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}ft", self.0)
    }
}

pub fn main() {
    let mut log = Log(Vec::new());
    dep::framed(&mut log, &Feet(-3)).unwrap();
    dep::framed(&mut log, &12u16).unwrap();
    println!("{:?}", log.0);
}
