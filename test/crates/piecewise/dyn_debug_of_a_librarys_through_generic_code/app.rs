//! Writes a library's `dyn Debug` into a writer of its own through its own
//! generic code: refused, naming #1797, as the library's contract says it
//! makes such a `dyn`, whose dictionary may not write a piece at a time,
//! never thrown as it's written (#1802).
use std::fmt::{self, Write};

struct Log(Vec<String>);

impl Write for Log {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.0.push(s.to_string());
        Ok(())
    }
}

fn own<W: Write, T: fmt::Debug>(w: &mut W, t: &T) -> fmt::Result {
    write!(w, "[{:?}]", t)
}

pub fn main() {
    let mut log = Log(Vec::new());
    own(&mut log, &dep::shown()).unwrap();
    println!("{:?}", log.0);
}
