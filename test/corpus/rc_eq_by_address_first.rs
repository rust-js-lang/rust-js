// `==` of two `Rc`s, or `Arc`s, of an `Eq` value asks whether they're one
// allocation first, as std's `RcEqIdent` specializes it, and calls the
// value's `eq` only of two; of a value that's only `PartialEq`, it calls
// `eq` always (#910).
use std::rc::Rc;
use std::sync::Arc;

#[derive(Debug)]
struct D(u32);

impl PartialEq for D {
    fn eq(&self, other: &D) -> bool {
        println!("eq {} {}", self.0, other.0);
        self.0 == other.0
    }
}

impl Eq for D {}

/// `Eq`, of `Rc`s whose counts nothing reads: each the value itself in JS.
#[derive(Debug)]
struct E(u32);

impl PartialEq for E {
    fn eq(&self, other: &E) -> bool {
        println!("e eq {} {}", self.0, other.0);
        self.0 == other.0
    }
}

impl Eq for E {}

/// Only `PartialEq`, whose `==` std always calls.
#[derive(Debug)]
struct P(u32);

impl PartialEq for P {
    fn eq(&self, other: &P) -> bool {
        println!("partial eq {} {}", self.0, other.0);
        self.0 == other.0
    }
}

fn main() {
    let a = Rc::new(D(1));
    let b = a.clone();
    let c = Rc::new(D(1));
    println!("{} {} {}", a == b, a == c, a != b);
    let items = vec![a.clone(), c.clone()];
    println!("{}", items == vec![b.clone(), c.clone()]);
    let x = Arc::new(D(2));
    println!("{}", x == x.clone());
    let p = Rc::new(P(3));
    println!("{}", p == p.clone());
    let e = Rc::new(E(4));
    println!("{} {}", e == e.clone(), e == Rc::new(E(4)));
    let f = Arc::new(E(5));
    println!("{}", f == f.clone());
    // One whose count the crate reads: a counted `Rc`.
    println!("{} {}", Rc::strong_count(&a), a == Rc::clone(&a));
}
