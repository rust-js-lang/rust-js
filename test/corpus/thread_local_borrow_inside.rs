//@ run-fail: RefCell already mutably borrowed
// A thread-local's `with_borrow_mut` holds its `RefCell` mutably borrowed
// while its closure runs, so a `with_borrow` inside panics, as std's does
// (ADR 0328).

use std::cell::RefCell;

thread_local! {
    static LOG: RefCell<Vec<String>> = RefCell::new(Vec::new());
}

fn len() -> usize {
    LOG.with_borrow(|log| log.len())
}

fn main() {
    LOG.with_borrow_mut(|log| log.push(len().to_string()));
    println!("{}", len());
}
