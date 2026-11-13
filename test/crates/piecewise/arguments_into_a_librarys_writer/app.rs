//! Gives the library's functions that write an `Arguments` into a writer
//! the `format_args!` it makes, each written a piece at a time; and writes
//! into the library's writer, whose own `write_fmt` passes it on, directly
//! and through generic code of its own.
use std::fmt::{self, Write};

struct Log(Vec<String>);

impl Write for Log {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.0.push(s.to_string());
        Ok(())
    }
}

fn own<W: Write>(w: &mut W, n: i32) -> fmt::Result {
    write!(w, "<{n:<3}>")
}

pub fn main() {
    let mut log = Log(Vec::new());
    let n = -4;
    dep::log_into(&mut log, format_args!("a{}b{:>3}", n, 'c')).unwrap();
    dep::relay(&mut log, format_args!("{:?}", "q")).unwrap();
    println!("{:?}", log.0);
    let mut tee = dep::Tee(Log(Vec::new()), 0);
    write!(tee, "{}-{}", n, 2).unwrap();
    own(&mut tee, 5).unwrap();
    println!("{:?} {}", tee.0 .0, tee.1);
    let mut counted = dep::Counted(Vec::new(), 0);
    write!(counted, "[{n:>3}]").unwrap();
    own(&mut counted, 6).unwrap();
    println!("{:?} {}", counted.0, counted.1);
}
