//! Writes the library's `&mut` and `&` of its type into a writer, directly,
//! with options, through generic code and through a `dyn`, a piece at a
//! time, then changes it through the `&mut` and formats it plainly (#1824).
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
    let mut m = dep::ByRef(7);
    let mut log = Log(Vec::new());
    {
        let mr = &mut m;
        write!(log, "{}|{:>4}|", mr, mr).unwrap();
        own(&mut log, &mr).unwrap();
        let d: &dyn fmt::Display = &mr;
        write!(log, "{d}").unwrap();
        mr.0 += 1;
        write!(log, "{}", mr).unwrap();
        println!("{}", mr);
    }
    let r = &m;
    write!(log, "{:?}", r).unwrap();
    println!("{:?} {}", log.0, m.0);
}
