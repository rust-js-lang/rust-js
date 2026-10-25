//! A crate given a library's boxed and counted `RefCell`s (ADR 0863): it
//! reads each as the `{ value }` the library made, and checks its own
//! borrows of one, so a borrow while the library holds one finds it
//! borrowed, as native Rust panics.
use std::panic::{AssertUnwindSafe, catch_unwind};

pub fn main() {
    let list = dep::shared(vec![1, 2]);
    list.borrow_mut().push(3);
    dep::each(&list, |n| print!("{n} "));
    println!("{}", list.borrow().len());
    let clash = catch_unwind(AssertUnwindSafe(|| {
        dep::each(&list, |n| list.borrow_mut().push(n));
    }));
    println!("{} {:?}", clash.is_err(), list.borrow());
}
