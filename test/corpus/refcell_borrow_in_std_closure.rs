//@ run-fail: RefCell already borrowed
// A borrow held while std's code runs a closure is held while the closure
// runs: one that borrows the cell again finds it borrowed, as std's does,
// so the cell's borrows are counted (ADRs 0328, 0863).

use std::cell::RefCell;

struct Log {
    items: RefCell<Vec<i32>>,
}

fn main() {
    let log = Log {
        items: RefCell::new(vec![1, 2]),
    };
    log.items.borrow_mut().push(3);
    println!("{}", log.items.borrow().len());
    let found = log.items.borrow().iter().any(|&item| {
        log.items.borrow_mut().push(item);
        false
    });
    println!("{found}");
}
