//! Writes a library's `dyn Debug` of a std value into a writer of its own,
//! a piece at a time, by its pair's dictionary's `fmtInto` (#1797).
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
