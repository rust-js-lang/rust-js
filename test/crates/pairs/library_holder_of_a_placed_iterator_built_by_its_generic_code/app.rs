//! A consumer giving the library's holder a fieldless enum's iterator, a
//! string in JS, and a unit struct's, `undefined`, each held as the
//! library's generic code holds it (#2255); and a tuple of them given and
//! given back by value, made into the library's `Zip`, and stepped by
//! the library's impl of its trait for a tuple.
use dep::{Step, Tools};

#[derive(Debug, Clone, Copy)]
enum Light {
    Red,
    Green,
}

impl Iterator for Light {
    type Item = u8;

    fn next(&mut self) -> Option<u8> {
        *self = match self {
            Light::Red => Light::Green,
            Light::Green => Light::Red,
        };
        Some(*self as u8)
    }
}

/// Gives sevens, a unit struct.
#[derive(Debug)]
struct Sevens;

impl Iterator for Sevens {
    type Item = u8;

    fn next(&mut self) -> Option<u8> {
        Some(7)
    }
}

pub fn main() {
    let chunks = Light::Red.chunks_of(3);
    println!("{:?} {:?} {}", chunks.next_chunk(), chunks.next_chunk(), chunks.taken());
    let sevens = dep::chunked(Sevens, 2);
    println!("{:?} {}", sevens.next_chunk(), sevens.taken());
    let (first, (light, sevens)) = dep::zipped((Light::Green, Sevens));
    println!("{first:?} {light:?} {sevens:?}");
    let (again, _) = dep::zipped((light, Light::Red));
    println!("{again:?}");
    let zipped: Vec<(u8, u8)> = dep::multizip((Light::Red, Sevens)).take(3).collect();
    println!("{zipped:?}");
    let (light, sevens) = (Light::Red, Sevens).step();
    println!("{light:?} {sevens:?}");
}
