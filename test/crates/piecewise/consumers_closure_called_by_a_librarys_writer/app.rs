//! Gives the library's writers a closure and a `fn` pointer of its own,
//! each given the `Formatter`, and writes them into a writer.
use std::fmt::{self, Write};

fn bracketed(f: &mut fmt::Formatter<'_>, n: i32) -> fmt::Result {
    write!(f, "[{n:>3}]")
}

pub fn main() {
    let k = 4;
    let via = dep::Via(bracketed, -3);
    let boxed = dep::Boxed(Box::new(move |f| write!(f, "<{}>", -k)));
    let mut chars = dep::Chars(Vec::new());
    write!(chars, "{}{}", via, boxed).unwrap();
    println!("{:?} {} {}", chars.0, via, boxed);
}
