//! Iterators JS holds as no object, a fieldless enum and a unit struct that
//! are their own iterators, given a library's generic code that clones or
//! copies them: it holds each in a place of its own, `{ value }` (#2255),
//! whose clone or copy is another place, never the place itself, so the
//! consumer compiles its own instance where the export takes no `Clone`
//! dictionary or copy function, as one crate's analysis gives them one
//! (#2540); the crate's iterator of a field and a range beside them. And
//! the crate's iterators as the items a library steps through their own
//! bound (#2541).
use dep::{cloned, copied, firsts, heads};

#[derive(Debug, Clone, Copy)]
enum Light {
    Red,
    Green,
}

impl Iterator for Light {
    type Item = u8;
    fn next(&mut self) -> Option<u8> {
        let was = *self as u8;
        *self = match self {
            Light::Red => Light::Green,
            Light::Green => Light::Red,
        };
        Some(was)
    }
}

#[derive(Debug, Clone, Copy)]
struct Empty;

impl Iterator for Empty {
    type Item = u8;
    fn next(&mut self) -> Option<u8> {
        None
    }
}

#[derive(Debug, Clone)]
struct Counter {
    n: u8,
}

impl Iterator for Counter {
    type Item = u8;
    fn next(&mut self) -> Option<u8> {
        self.n += 1;
        Some(self.n)
    }
}

pub fn main() {
    println!("{:?}", cloned(Light::Red));
    println!("{:?}", cloned(Empty));
    println!("{:?}", cloned(Counter { n: 0 }));
    println!("{:?}", copied(Light::Green));
    println!("{:?}", copied(Empty));
    println!("{:?}", firsts(Light::Red));
    println!("{:?}", firsts(Counter { n: 3 }));
    println!("{:?}", firsts(0..4u8));
    println!("{:?}", heads([Counter { n: 0 }, Counter { n: 5 }].into_iter()));
    println!("{:?}", heads(vec![0..2u8, 5..9].into_iter()));
}
