//! Writes a library's `dyn Debug` of a std value whose dictionary it left
//! without `fmtInto` into a writer of its own: refused, naming #1797.
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
