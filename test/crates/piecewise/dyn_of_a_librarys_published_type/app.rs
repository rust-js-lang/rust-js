//! Writes the library's `dyn Display` and `dyn Error` into a writer of its
//! own a piece at a time, each piece a write of its own, as
//! native does (#1688).
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
    write!(log, "<{}>", dep::boxed(4)).unwrap();
    let err = dep::parse(12).unwrap_err();
    write!(log, "[{err}]").unwrap();
    println!("{:?}", log.0);
}
