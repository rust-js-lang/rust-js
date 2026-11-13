// A `write!` into a `String` through generic code's `W: fmt::Write`, or a
// `dyn fmt::Write`, keeps what it wrote before a value whose `Display`
// fails, as std's `write_fmt` writes each piece before it (#1696); and so
// does an `Arguments` given on to such a writer.
use std::fmt::{self, Write};

struct Refused;

impl fmt::Display for Refused {
    fn fmt(&self, _: &mut fmt::Formatter<'_>) -> fmt::Result {
        Err(fmt::Error)
    }
}

fn put<W: Write>(w: &mut W, n: i32) -> fmt::Result {
    write!(w, "x{n}y{}", Refused)
}

fn into_any<W: Write>(w: &mut W, args: fmt::Arguments<'_>) -> fmt::Result {
    w.write_fmt(args)
}

fn into_dyn(w: &mut dyn Write, n: i32) -> fmt::Result {
    write!(w, "[{n:>3}]{}", Refused)
}

fn main() {
    let mut s = String::from(">");
    let r = put(&mut s, 1);
    println!("{s:?} {r:?}");
    let mut s = String::new();
    let r = into_any(&mut s, format_args!("a{}b{}", 2, Refused));
    println!("{s:?} {r:?}");
    let mut s = String::new();
    let r = into_dyn(&mut s, -3);
    println!("{s:?} {r:?}");
    let mut s = String::new();
    let r = put(&mut &mut s, 4);
    println!("{s:?} {r:?}");
}
