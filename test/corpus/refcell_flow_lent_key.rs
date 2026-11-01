//@ run-fail: RefCell already mutably borrowed
// A thread-local's cell lent by `with` is each borrow of what it lends
// (ADRs 0863, 0960, amended): its `with_borrow_mut` held while a function
// borrows the lent cell is checked, and the borrow panics, as std's does.

use std::cell::RefCell;

thread_local! {
    static LOG: RefCell<Vec<i32>> = const { RefCell::new(Vec::new()) };
}

fn count() -> usize {
    LOG.with(|log| log.borrow().len())
}

fn main() {
    LOG.with_borrow_mut(|log| log.push(1));
    println!("{}", count());
    LOG.with_borrow_mut(|log| {
        log.push(2);
        println!("{}", count());
    });
}
