//@ run-fail: RefCell already mutably borrowed
// A function pointer made of a closure that borrows a cell held while
// it's called: the borrow is counted, and the closure's panics, as std's
// does (ADR 0903).

use std::cell::RefCell;

struct Root {
    log: RefCell<Vec<usize>>,
}

fn record(root: &Root, f: fn(&Root) -> usize) {
    let mut log = root.log.borrow_mut();
    let n = f(root);
    log.push(n);
}

fn main() {
    let root = Root {
        log: RefCell::new(Vec::new()),
    };
    let counted: fn(&Root) -> usize = |root| root.log.borrow().len();
    let constant: fn(&Root) -> usize = |_| 7;
    record(&root, constant);
    println!("{:?}", root.log.borrow());
    println!("{}", counted(&root));
    record(&root, counted);
}
