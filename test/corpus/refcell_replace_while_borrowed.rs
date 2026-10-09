//@ run-fail: RefCell already borrowed
// `replace` borrows its cell mutably, so it panics while a `borrow()` lives,
// as std's does (ADR 0328).

use std::cell::RefCell;

fn main() {
    let cell = RefCell::new(1);
    let reading = cell.borrow();
    cell.replace(2);
    println!("{}", *reading);
}
