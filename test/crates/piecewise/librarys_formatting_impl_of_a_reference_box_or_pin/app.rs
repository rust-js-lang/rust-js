//! Writes the library's types by its impls of a reference, a `Box` and a
//! `Pin<Box<..>>` of them into a writer, directly, through generic code and
//! through a `dyn`, a piece at a time, and formats them plainly.
use std::fmt::{self, Write};

struct Log(Vec<String>);

impl Write for Log {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.0.push(s.to_string());
        Ok(())
    }
}

fn own<W: Write, T: fmt::Display>(w: &mut W, t: &T) -> fmt::Result {
    write!(w, "[{}]", t)
}

pub fn main() {
    let r = dep::ByRef(1);
    let b = Box::new(dep::Boxed(2));
    let p = Box::pin(dep::Pinned(3));
    let mut log = Log(Vec::new());
    write!(log, "{}{:?}", &r, &r).unwrap();
    write!(log, "{}", b).unwrap();
    write!(log, "{}", p).unwrap();
    own(&mut log, &&r).unwrap();
    own(&mut log, &b).unwrap();
    own(&mut log, &p).unwrap();
    let dyns: [&dyn fmt::Display; 3] = [&&r, &b, &p];
    for d in dyns {
        write!(log, "{d}").unwrap();
    }
    println!("{:?}", log.0);
    println!("{} {:#?} [{:>4}] {}", &r, &r, b, p.to_string());
}
