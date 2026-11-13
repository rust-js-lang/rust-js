//! Writes a library's `dyn Debug` into a writer of its own: refused,
//! naming #1688, as the library's contract says it makes such a `dyn`,
//! whose dictionary may not write a piece at a time.
use std::fmt::{self, Write};

struct Log(Vec<String>);

impl Write for Log {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.0.push(s.to_string());
        Ok(())
    }
}

pub fn main() {
    let mut log = Log(Vec::new());
    write!(log, "{:?}", dep::shown()).unwrap();
    println!("{:?}", log.0);
}
