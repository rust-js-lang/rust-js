//@ run-fail: RefCell already borrowed
// A thread-local `RefCell`'s `take` borrows it mutably (ADR 0595,
// amended): taken while a borrow of it is held, it panics, as std's does.
use std::cell::RefCell;

thread_local! {
    static PORTS: RefCell<Vec<u32>> = const { RefCell::new(Vec::new()) };
}

fn main() {
    PORTS.set(vec![1, 2]);
    let total = PORTS.with_borrow(|ports| {
        let first = ports[0];
        let taken = PORTS.take();
        first + taken.len() as u32
    });
    println!("{total}");
}
