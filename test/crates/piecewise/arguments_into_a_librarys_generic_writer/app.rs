//! Gives the library's generic writer a `format_args!` of its own, whose
//! `T` the library writes into the writer through its dictionary: the
//! function that writes into the sink it claims, each of its pieces a
//! write of its own, as std writes them (#1745).
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
    let n = 1;
    dep::framed(&mut log, &format_args!("a{}b", n)).unwrap();
    println!("{:?}", log.0);
}
