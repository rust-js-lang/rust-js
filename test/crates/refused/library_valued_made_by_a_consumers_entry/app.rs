//! A consumer's impl of a library's trait whose method gives a write's
//! result where the trait gives `Self::Out`, given to the library's generic
//! code, which holds it as a value: refused where the dictionary is given,
//! as one crate refuses it (#2559), until the entry gives its result as a
//! value (#2565).
use dep::{Make, made_twice};
use std::fmt;

pub struct Bad;

impl Make for Bad {
    type Out = fmt::Result;
    fn make(&self) -> fmt::Result {
        Err(fmt::Error)
    }
}

pub fn main() {
    let (a, b) = made_twice(&Bad);
    println!("{a:?} {b:?} {}", a.is_err());
}
