//! Writes a `dyn Display` of its own into a writer a piece at a time,
//! beside a library whose derived `Debug` makes `&dyn Debug`s it keeps.
use std::fmt::{self, Write};

struct Log(Vec<String>);

impl Write for Log {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.0.push(s.to_string());
        Ok(())
    }
}

struct Point(i32, i32);

impl fmt::Display for Point {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({}, {})", self.0, self.1)
    }
}

pub fn main() {
    let shown: [&dyn fmt::Display; 2] = [&Point(1, -2), &7];
    let mut log = Log(Vec::new());
    for value in shown {
        write!(log, "{value}|").unwrap();
    }
    println!("{:?} {}", log.0, dep::described(-1));
}
