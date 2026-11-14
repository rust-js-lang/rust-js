//! Writes a library's `dyn Debug` of its type parameter's value, the
//! consumer's own, into a writer of its own: refused, naming #1797.
use std::fmt::{self, Write};

struct Log(Vec<String>);

impl Write for Log {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.0.push(s.to_string());
        Ok(())
    }
}

struct Mine(i32);

impl fmt::Debug for Mine {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "m{}", self.0)
    }
}

pub fn main() {
    let mut log = Log(Vec::new());
    write!(log, "{:?}", dep::boxed(Mine(3))).unwrap();
    println!("{:?}", log.0);
}
