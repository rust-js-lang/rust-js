//@ compile-fail: `==` of two `std::rc::Rc<Kind>`s, which std compares by address first, of a value that isn't an object in JS
// `==` of two `Rc`s in generic code asks whether they're one allocation
// first where the caller's `T` is `Eq`, `TEq` (#931). An `Rc` whose counts
// nothing reads is its value in JS, and a fieldless enum's value is its
// name's string, which two allocations share: the caller giving it is
// refused, as a `==` of its own is (#930).
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

fn same<T: PartialEq>(a: &Rc<T>, b: &Rc<T>) -> bool {
    a == b
}

fn main() {
    let a = Rc::new(Kind::A);
    let b = Rc::new(Kind::A);
    println!("{} {:?}", same(&a, &b), Kind::B);
}
