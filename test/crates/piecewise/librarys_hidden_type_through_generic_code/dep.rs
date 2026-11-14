//! A library's types no consumer can name, which it gives out as an `impl
//! Display`, one through a private function's: each is published as a type
//! reachable from outside it is, as a consumer writes it into a writer as
//! that type (#1743).
use std::fmt;

struct Hidden(i32);

impl fmt::Display for Hidden {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "<{}>", self.0)
    }
}

struct Inner(char);

impl fmt::Display for Inner {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({})", self.0)
    }
}

pub fn hidden(n: i32) -> impl fmt::Display {
    Hidden(n)
}

fn inner(c: char) -> impl fmt::Display {
    Inner(c)
}

pub fn wrapped(c: char) -> impl fmt::Display {
    inner(c)
}
