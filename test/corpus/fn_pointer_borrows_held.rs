//@ run-fail: RefCell already borrowed
// A function pointer that may be a function, or a closure, that borrows a
// cell held while it's called: the borrow is counted, and the second one
// panics, as std's does (ADR 0907).

use std::cell::RefCell;

struct Root {
    log: RefCell<Vec<usize>>,
}

fn quiet(_root: &Root) -> usize {
    0
}

fn touch(root: &Root) -> usize {
    root.log.borrow_mut().push(1);
    1
}

fn report(root: &Root, f: fn(&Root) -> usize) -> usize {
    let log = root.log.borrow();
    log.len() + f(root)
}

fn main() {
    let root = Root {
        log: RefCell::new(Vec::new()),
    };
    let pushed: fn(&Root) -> usize = |root| {
        root.log.borrow_mut().push(2);
        2
    };
    println!("{}", touch(&root) + pushed(&root));
    let reports: [fn(&Root) -> usize; 3] = [quiet, quiet, touch];
    for f in reports {
        println!("{}", report(&root, f));
    }
}
