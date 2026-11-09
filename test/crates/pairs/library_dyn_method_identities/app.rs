//! Two `Rc`s a library's `dyn Trait` method makes, compared: apart, as Rust
//! keeps them (#1055).
use dep::made;
use std::rc::Rc;

pub fn main() {
    let twice = made(5);
    let (first, second) = twice.pair();
    let again = Rc::clone(&first);
    println!("{} {} {} {}", first(), second(), Rc::ptr_eq(&first, &second), Rc::ptr_eq(&first, &again));
}
