//! Writes the library's `dyn Error` into a writer a piece at a time, which
//! is refused, naming #1688: its dictionary may carry no `fmtInto`.
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
    let err = dep::parse(12).unwrap_err();
    write!(log, "[{err}]").unwrap();
    println!("{:?}", log.0);
}
