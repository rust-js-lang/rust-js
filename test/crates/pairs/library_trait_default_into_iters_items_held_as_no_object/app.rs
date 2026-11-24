//! A consumer giving the library's trait default an iterator of iterators
//! JS holds as no object: a unit struct's, which panics if stepped, so its
//! product is lazy, and a fieldless enum's, stepped through the place its
//! library holds it in (#2466).
use dep::Tools;

#[derive(Debug, Clone)]
struct Lazy;

impl Iterator for Lazy {
    type Item = u8;

    fn next(&mut self) -> Option<u8> {
        panic!("stepped")
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (0, Some(0))
    }
}

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
    let lazy = vec![Lazy, Lazy, Lazy].into_iter().product_of();
    println!("{:?}", lazy.hints());
    let mut lights = [Light::Red, Light::Green].into_iter().product_of();
    println!("{:?}", lights.firsts());
    println!("{:?}", lights.firsts());
    println!("{:?}", lights.hints());
}
