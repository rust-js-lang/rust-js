//! Writes the library's type into a writer through generic code: refused,
//! naming #1688, as the library's contract doesn't say its dictionary can.
use std::fmt::{self, Write};

fn own<W: Write, T: fmt::Display>(w: &mut W, t: &T) -> fmt::Result {
    write!(w, "<{}>", t)
}

pub fn main() {
    let mut text = String::new();
    own(&mut text, &dep::Meters(6)).unwrap();
    println!("{}", text);
}
