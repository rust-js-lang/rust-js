//! Gives one library's `dyn Debug` to another library that writes it into
//! a writer a piece at a time: refused, naming #1797, as the first's
//! contract says it makes such a `dyn`, whose dictionary may not write a
//! piece at a time, never thrown as it's written (#1802).
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
    let shown = dep::shown();
    mid::show(&mut log, &*shown).unwrap();
    println!("{:?}", log.0);
}
