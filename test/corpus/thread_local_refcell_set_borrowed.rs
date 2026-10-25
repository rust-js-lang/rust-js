//@ run-fail: RefCell already borrowed
// A thread-local `RefCell`'s `set` borrows it mutably to replace what it
// holds (issue #249): set while a borrow of it is held, it panics, as
// std's does, of one that's the array it holds (ADR 0863).
use std::cell::RefCell;

thread_local! {
    static PORTS: RefCell<Vec<u32>> = const { RefCell::new(Vec::new()) };
}

fn main() {
    PORTS.set(vec![1, 2]);
    let total = PORTS.with_borrow(|ports| {
        let first = ports[0];
        PORTS.set(vec![9]);
        first + ports[1]
    });
    println!("{total}");
}
