//! Names the library's writer functions as `fn` pointers, which its framed
//! writer calls, written into a writer a piece at a time and printed.
use dep::{Via, bracketed, plain};
use std::fmt::{self, Write};

struct Log(Vec<String>);

impl Write for Log {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.0.push(s.to_string());
        Ok(())
    }
}

pub fn main() {
    let via = Via(bracketed, -3);
    let other = Via(plain, 4);
    let mut log = Log(Vec::new());
    write!(log, "{via}{other}").unwrap();
    println!("{via} {other} {:?}", log.0);
}
