//@ run-fail: RefCell already borrowed
// A `RefCell`'s `take` while it's borrowed: std's `take` makes its
// `Default` first, which prints, and only then borrows the cell, which
// panics. So the JS prints what `Default` does before it panics (#699).
use std::cell::RefCell;

#[derive(Debug)]
struct Log(u32);

impl Default for Log {
    fn default() -> Self {
        println!("log default");
        Log(0)
    }
}

fn main() {
    let log = RefCell::new(Log(1));
    let old = log.take();
    println!("took {:?}, left {:?}", old, log.borrow());
    let held = log.borrow();
    let again = log.take();
    println!("took again {:?} {:?}", again, *held);
}
