//@ run-fail: RefCell already borrowed
// A borrow an `if let` tests is held while its `then` runs, as edition
// 2024 holds the temporaries of what it tests: a `borrow_mut()` of the same
// cell there finds it borrowed, as std's does (ADRs 0328, 0863).

use std::cell::RefCell;

fn main() {
    let next = RefCell::new(Some(1));
    if let Some(n) = *next.borrow() {
        println!("tested {n}");
    }
    println!("{:?}", next.borrow());
    if let Some(n) = *next.borrow() {
        next.borrow_mut().replace(n + 1);
    }
    println!("{:?}", next.borrow());
}
