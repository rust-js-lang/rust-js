//@ library-differs
// A generic type's writer, written into a writer a piece at a time, whose
// parameters are written by their dictionaries, `$writeBy`, with width
// options: written into a writer, by its pieces; formatted plainly and
// into a `String` through generic code, by their text, padded as std pads
// it, a type of the crate's that ignores the width too (#1712).
use std::fmt::{self, Write};

struct Log(Vec<String>);

impl Write for Log {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.0.push(s.to_string());
        Ok(())
    }
}

struct Duo<T>(T, T);

impl<T: fmt::Display> fmt::Display for Duo<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({}, {:>3})", self.0, self.1)
    }
}

struct Tag;

impl fmt::Display for Tag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("t")
    }
}

fn shown<W: Write, T: fmt::Display>(w: &mut W, t: T) {
    write!(w, "[{:>4}|{:<3}]", t, t).unwrap();
}

fn main() {
    let mut log = Log(Vec::new());
    write!(log, "{}", Duo(1, 2)).unwrap();
    println!("{:?}", log.0);
    println!("{}", Duo(7, 8));
    println!("{}", Duo("a", "b"));
    println!("{}", Duo(Tag, Tag));
    println!("{}", Duo(1.5, 2.25));
    let mut s = String::new();
    shown(&mut s, 5);
    shown(&mut s, "x");
    shown(&mut s, Tag);
    shown(&mut s, 'c');
    println!("{s}");
    shown(&mut log, 9);
    println!("{:?}", log.0);
}
