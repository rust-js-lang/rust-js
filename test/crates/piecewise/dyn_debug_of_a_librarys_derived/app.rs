//! Writes the library's `dyn Debug` of a derived `Debug` into a writer of
//! its own a piece at a time: refused, naming #1797, as the library's
//! contract says the `dyn` may carry no `fmtInto`.
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
