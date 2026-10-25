// A closure the program wrote as its call's value, `|| f()`, is `() =>
// f()` where the call gives `()`, whose JS gives `undefined`: a function of
// the crate's, a closure of its own; its comment after the `=>` (ADR 0883).
// One written `|| { f(); }` stays a block, and one of a call whose JS gives
// a value, a `Vec`'s `push`, too. What each gives is read, `()` each.
// And a blank line before an `if` whose test has a comment after the `if`,
// its statement's or the `return` it's the value of,
// and a `RefCell`'s `Vec` written through its `borrow_mut()`, which is the
// array itself.

use std::cell::RefCell;
use std::fmt::Debug;

fn record(log: &RefCell<Vec<String>>, what: &str) {
    log.borrow_mut().push(what.to_string());
}

fn later<T: Debug>(f: impl FnOnce() -> T) -> String {
    format!("{:?}", f())
}

fn twice(f: &dyn Fn()) {
    f();
    f();
}

fn flush(queue: &RefCell<Vec<u32>>, at: usize, task: u32) -> usize {
    let len = queue.borrow().len();
    if at > len {
        return 0;
    }

    if
    // A task past the queue's end is pushed.
    at == len && task > 0
    {
        queue.borrow_mut().push(task);
        return 1;
    }
    queue.borrow_mut()[at] = task;
    2
}

fn picked(o: Option<u32>, n: u32) -> u32 {
    if n > 5 {
        println!("big");
    }

    if
    // Its value, tested so.
    let Some(r) = o
        && r > 3
    {
        r
    } else {
        n
    }
}

fn main() {
    let log = RefCell::new(Vec::new());
    let mut given = Vec::new();
    given.push(later(|| {
        // Recorded first.
        record(&log, "commented")
    }));
    given.push(later(|| record(&log, "value")));
    given.push(later(|| {
        record(&log, "statement");
    }));
    let inner = || record(&log, "inner");
    given.push(later(|| inner()));
    given.push(later(|| twice(&|| record(&log, "twice"))));
    let mut pushed = Vec::new();
    given.push(later(|| pushed.push(7)));
    println!("{given:?} {:?} {pushed:?}", log.borrow());

    let queue = RefCell::new(vec![1, 2]);
    let ways = [flush(&queue, 1, 5), flush(&queue, 2, 6), flush(&queue, 9, 1)];
    println!("{ways:?} {:?}", queue.borrow());
    println!("{} {}", picked(Some(4), 6), picked(None, 2));
}
