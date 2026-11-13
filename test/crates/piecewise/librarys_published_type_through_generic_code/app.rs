//! Writes the library's type into a writer through generic code, and
//! directly into a writer of its own, a piece at a time by the dictionary
//! its library publishes.
use std::fmt::{self, Write};

struct Log(Vec<String>);

impl Write for Log {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.0.push(s.to_string());
        Ok(())
    }
}

fn own<W: Write, T: fmt::Display>(w: &mut W, t: &T) -> fmt::Result {
    write!(w, "<{}>", t)
}

pub fn main() {
    let mut text = String::new();
    own(&mut text, &dep::Meters(6)).unwrap();
    let mut log = Log(Vec::new());
    own(&mut log, &dep::Meters(7)).unwrap();
    write!(log, "{}|", dep::Meters(8)).unwrap();
    println!("{} {:?}", text, log.0);
}
