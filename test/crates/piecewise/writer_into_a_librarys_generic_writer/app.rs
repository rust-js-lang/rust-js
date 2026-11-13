//! Gives the library's generic writer a type of its own and a std one, each
//! written a piece at a time through the dictionary it gives; and writes
//! the library's type through generic code of its own, by the dictionary
//! the library's contract says carries `fmtInto`.
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

fn own<W: Write, T: fmt::Display>(w: &mut W, t: &T) -> fmt::Result {
    write!(w, "<{:<5}>", t)
}

pub fn main() {
    let mut log = Log(Vec::new());
    dep::framed(&mut log, &Feet(-3)).unwrap();
    dep::framed(&mut log, &-7i8).unwrap();
    dep::framed(&mut log, &"ab").unwrap();
    dep::meters_into(&mut log, &dep::Meters(5)).unwrap();
    own(&mut log, &dep::Meters(6)).unwrap();
    println!("{:?}", log.0);
}
