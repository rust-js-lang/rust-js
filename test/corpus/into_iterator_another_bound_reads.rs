// An `IntoIterator` type parameter another bound reads as its own type,
// `T: IntoIterator + Clone`'s `t.clone()`, `T: IntoIterator + Debug`'s
// `{t:?}`, `Copy`'s copy: its caller's own value, given as it is with its
// `IntoIterator` dictionary, as an `Iterator` another bound reads is (ADR
// 0474), never the JS iterator a caller makes of it (#2540). A fieldless
// enum and a unit struct that are their own iterators, held in a place of
// their own (#2255), whose copy is another place, the crate's iterator of a
// field, a range, std's iterators and a `Vec`; each clone or copy steps
// apart from what it's made of.

use std::fmt::Debug;

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

fn firsts<T: IntoIterator + Clone>(t: T) -> Vec<Option<T::Item>> {
    let mut a = t.clone().into_iter();
    let mut b = t.into_iter();
    vec![a.next(), b.next(), a.next()]
}

fn by_ref_clone<T: IntoIterator + Clone>(t: &T) -> Vec<Option<T::Item>> {
    let mut a = t.clone().into_iter();
    vec![a.next(), a.next()]
}

fn copied<T: Iterator + Copy>(t: T) -> Vec<Option<T::Item>> {
    let mut a = t;
    let mut b = t;
    vec![a.next(), b.next(), a.next()]
}

fn copied_into<T: IntoIterator + Copy>(t: T) -> Vec<Option<T::Item>> {
    let mut a = t.into_iter();
    let mut b = t.into_iter();
    vec![a.next(), b.next(), a.next()]
}

fn shown<T: IntoIterator + Debug>(t: T) -> String {
    let s = format!("{t:?}");
    let n = t.into_iter().take(5).count();
    format!("{s} {n}")
}

fn same<T: IntoIterator + PartialEq + Clone>(t: T) -> bool {
    let u = t.clone();
    let mut a = t.into_iter();
    a.next();
    u == u.clone()
}

fn main() {
    println!("{:?}", firsts(Light::Red));
    println!("{:?}", firsts(Empty));
    println!("{:?}", firsts(Counter { n: 0 }));
    println!("{:?}", firsts(0..5u8));
    println!("{:?}", firsts(vec![1u8, 2, 3]));
    println!("{:?}", firsts(vec![1u8, 2, 3].into_iter()));
    println!("{:?}", firsts("abc".chars()));
    println!("{:?}", by_ref_clone(&Light::Green));
    println!("{:?}", by_ref_clone(&Counter { n: 7 }));
    println!("{:?}", copied(Light::Green));
    println!("{:?}", copied(Empty));
    println!("{:?}", copied_into(Light::Red));
    println!("{:?}", copied_into(&[1u8, 2][..]));
    println!("{}", shown(0..3u8));
    println!("{}", shown(Counter { n: 250 }.take(2)));
    println!("{}", shown(Light::Red.take(3)));
    println!("{}", shown(vec![1u8, 2]));
    println!("{}", same(0..3u8));
    println!("{}", same(vec![1u8]));
}
