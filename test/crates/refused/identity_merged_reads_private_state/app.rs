//! Refused, saying why `made` is compiled here.
use std::rc::Rc;

pub fn main() {
    let (first, second) = dep::made();
    println!("{} {}", first(), Rc::ptr_eq(&first, &second));
}
