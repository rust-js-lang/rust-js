//! Iterators JS holds as no object, a fieldless enum that is its own
//! iterator and a unit struct whose `IntoIter` is a unit struct iterator,
//! given a library's struct by each bound: generic code holds them in a
//! place of its own, `{ value }`, and what `into_iter()` makes is one too
//! (#2508, #2520). A trait default given them as its `Self`'s items too is
//! #2534's.
use dep::{Held, Tools, held_of};

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
struct Bag;

#[derive(Debug, Clone)]
struct Empty;

impl Iterator for Empty {
    type Item = u8;
    fn next(&mut self) -> Option<u8> {
        None
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        (0, Some(0))
    }
}

impl IntoIterator for Bag {
    type Item = u8;
    type IntoIter = Empty;
    fn into_iter(self) -> Empty {
        Empty
    }
}

pub fn main() {
    let mut a = held_of(vec![Light::Red, Light::Green]);
    println!("held_of {:?} {:?}", a.firsts(), a.firsts());
    println!("held_of {:?}", held_of([Bag, Bag]).hints());
    let mut c = Held::new(Light::Green).with(Light::Red);
    println!("with {:?} {:?}", c.firsts(), c.firsts());
    println!("with {:?}", Held::new(Empty).with(Bag).hints());
    println!("held_with {:?}", [Bag].into_iter().held_with(Bag).hints());
    for mut h in [Light::Red, Light::Green].into_iter().nested() {
        println!("nested {:?}", h.firsts());
    }
    for h in [Bag, Bag].into_iter().nested() {
        println!("nested {:?}", h.hints());
    }
}
