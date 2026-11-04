// A borrow its statement holds meets nothing that ran before it was made,
// in Rust's order of evaluation (#552): `format_args!` evaluates `held(&log,
// 2)` before `log.borrow()`, so no borrow `held` makes can meet the `Ref`,
// though the `Ref` lives to the statement's end. No borrow of the cell is
// counted, so the cell is the array it holds.

use std::cell::RefCell;

fn held(log: &RefCell<Vec<u32>>, n: u32) -> usize {
    log.borrow_mut().push(n);
    log.borrow().len()
}

fn apart() {
    let log = RefCell::new(Vec::new());
    log.borrow_mut().push(1);
    let n = held(&log, 2);
    println!("{} {:?}", n, log.borrow());
}

fn together() {
    let log = RefCell::new(Vec::new());
    log.borrow_mut().push(1);
    println!("{} {:?}", held(&log, 2), log.borrow());
}

fn sum(a: usize, b: usize) -> usize {
    a + b
}

fn in_a_call() {
    let log = RefCell::new(vec![1]);
    let total = sum(held(&log, 2), log.borrow().len());
    println!("{total} {:?}", (held(&log, 3), log.borrow().len()));
}

fn scaled(by: usize, n: usize) -> usize {
    n * by
}

fn picked(log: &RefCell<Vec<u32>>) -> fn(usize, usize) -> usize {
    log.borrow_mut().push(0);
    scaled
}

fn in_parts() {
    let log = RefCell::new(vec![1]);
    let total = held(&log, 2) + log.borrow().len();
    let both = held(&log, 3) > 2 && log.borrow().len() > 2;
    let lens = [held(&log, 4), log.borrow().len()];
    let doubled = picked(&log)(2, log.borrow().len());
    println!("{total} {both} {lens:?} {doubled}");
}

fn in_a_loop() {
    let log = RefCell::new(vec![1]);
    for n in 2..4 {
        println!("{} {:?}", held(&log, n), log.borrow());
    }
}

fn main() {
    apart();
    together();
    in_a_call();
    in_parts();
    in_a_loop();
}
