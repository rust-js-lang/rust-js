//! A library's trait default that holds its own `J` in a place of its own,
//! given a fieldless enum that is its own iterator, and given values of it
//! as its `Self`'s items, which come as their caller holds them: refused at
//! the call, as one crate refuses it (#2534, #2508's form 5), until each
//! item is made a place where it crosses (#2535).
use dep::Tools;

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

pub fn main() {
    let mut e = [Light::Green, Light::Red].into_iter().held_with(Light::Red);
    println!("held_with {:?} {:?}", e.firsts(), e.firsts());
}
