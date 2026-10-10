//@ compile-fail: the `TypeId` of a closure's type
// A closure's type has no name of its own, which a `TypeId` is in JS: two
// closures share one (ADR 0331).

use std::any::Any;

fn main() {
    let add = |x: i32| x + 1;
    let any: &dyn Any = &add;
    println!("{}", any.is::<i32>());
}
