//! Names the library's generic writer function as a `fn` pointer at an
//! instance and calls it through the pointer: its arrow takes what the
//! library's JS takes, no `Formatter`, and gives its dictionary (#1710).
use std::fmt;

type Writing = fn(&mut fmt::Formatter<'_>, i32) -> fmt::Result;

struct Via(Writing, i32);

impl fmt::Display for Via {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        (self.0)(f, self.1)
    }
}

pub fn main() {
    println!("{}", Via(dep::bracketed, -3));
}
