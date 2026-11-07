//@ compile-fail: `==` of two `std::rc::Rc<Kind>`s, which std compares by address first, of a value that isn't an object in JS
// `==` of two `Rc`s of an `Eq` value is the same allocation or its value's
// `==`, std's `RcEqIdent` (#910). An `Rc` whose counts nothing reads is its
// value in JS, and a fieldless enum's value is its name's string, which
// two allocations share: refused, where its own `eq` would be skipped.
use std::rc::Rc;

#[derive(Debug, Clone, Copy)]
enum Kind {
    A,
    B,
}

impl PartialEq for Kind {
    fn eq(&self, other: &Kind) -> bool {
        println!("eq");
        matches!((self, other), (Kind::A, Kind::A) | (Kind::B, Kind::B))
    }
}

impl Eq for Kind {}

fn main() {
    let a = Rc::new(Kind::A);
    let b = Rc::new(Kind::A);
    println!("{} {:?}", a == b, Kind::B);
}
