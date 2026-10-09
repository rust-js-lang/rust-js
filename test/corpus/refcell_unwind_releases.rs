//@ run-fail: boom
// A panic in a thread-local's `with_borrow_mut` closure unwinds, releasing
// the borrow before a destructor that asks runs, as std's does (ADR 0328).

use std::cell::RefCell;

thread_local! {
    static LOG: RefCell<Vec<i32>> = RefCell::new(Vec::new());
}

struct Asks;

impl Drop for Asks {
    fn drop(&mut self) {
        LOG.with(|log| println!("free as it unwinds: {}", log.try_borrow_mut().is_ok()));
    }
}

fn main() {
    let _asks = Asks;
    LOG.with_borrow_mut(|log| {
        log.push(1);
        if log.len() == 1 {
            panic!("boom");
        }
    });
}
