//! Writes the library's generic type into a writer through generic code,
//! which is refused, naming #1688, as its dictionary can't write it a piece
//! at a time.
use std::fmt::{self, Write};

struct Log(Vec<String>);

impl Write for Log {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.0.push(s.to_string());
        Ok(())
    }
}

fn own<W: Write, T: fmt::Display>(w: &mut W, t: &T) -> fmt::Result {
    write!(w, "[{}]", t)
}

pub fn main() {
    let mut log = Log(Vec::new());
    own(&mut log, &dep::Wrap(3)).unwrap();
    println!("{:?}", log.0);
}
