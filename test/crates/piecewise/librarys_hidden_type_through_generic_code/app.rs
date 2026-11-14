//! Writes the library's hidden types into a writer through generic code
//! and directly, a piece at a time, and formats them plainly.
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
    own(&mut log, &dep::hidden(3)).unwrap();
    own(&mut log, &dep::wrapped('w')).unwrap();
    write!(log, "{}", dep::wrapped('d')).unwrap();
    println!("{:?}", log.0);
    println!("{} {}", dep::hidden(4), dep::wrapped('p'));
}
