//@ library-differs
// A `RefCell`'s length, known across a `for` loop's steps: its `next()`,
// of a range of integers, a `step_by(2)` of one or a slice's enumerated
// iterator, is std's and runs none of the crate's code, so `batch[i]`
// under `i` below `batch.borrow().len()` reads with no check, the length
// held in a `let` or the range's own (ADR 1168, #1948). What only looks
// alike keeps its check and panics as Rust's does: an index past the end,
// `0..=len` or `i + 1`; a held length past a `clear()`, a push, or a call
// of the crate's that empties the cell in the body; and the range's own
// borrow, which the body's `borrow_mut()` panics at, held the whole loop.
use std::cell::RefCell;
use std::panic::{self, AssertUnwindSafe};

fn stepped(batch: &RefCell<Vec<u32>>) -> u32 {
    let mut total = 0;
    for i in (0..batch.borrow().len()).step_by(2) {
        total += batch.borrow()[i];
    }
    total
}

fn held(batch: &RefCell<Vec<u32>>) -> u32 {
    let mut total = 0;
    let end = batch.borrow().len();
    for i in (0..end).step_by(2) {
        total += batch.borrow()[i];
    }
    total
}

fn each(batch: &RefCell<Vec<u32>>) -> u32 {
    let mut total = 0;
    for i in 0..batch.borrow().len() {
        total += batch.borrow()[i] * 10;
    }
    total
}

fn enumerated(batch: &RefCell<Vec<u32>>) -> u32 {
    let mut total = 0;
    for (i, item) in batch.borrow().iter().enumerate() {
        total += batch.borrow()[i] + item;
    }
    total
}

fn held_grown(batch: &RefCell<Vec<u32>>) -> u32 {
    let mut total = 0;
    let end = batch.borrow().len();
    for i in (0..end).step_by(2) {
        batch.borrow_mut().push(9);
        total += batch.borrow()[i];
    }
    total
}

fn held_cleared(batch: &RefCell<Vec<u32>>) -> u32 {
    let mut total = 0;
    let end = batch.borrow().len();
    for i in 0..end {
        total += batch.borrow()[i];
        batch.borrow_mut().clear();
    }
    total
}

fn empty(batch: &RefCell<Vec<u32>>) {
    batch.borrow_mut().clear();
}

fn held_emptied_by_a_call(batch: &RefCell<Vec<u32>>) -> u32 {
    let mut total = 0;
    let end = batch.borrow().len();
    for i in 0..end {
        total += batch.borrow()[i];
        empty(batch);
    }
    total
}

fn past_the_end(batch: &RefCell<Vec<u32>>) -> u32 {
    let mut total = 0;
    for i in (0..=batch.borrow().len()).step_by(2) {
        total += batch.borrow()[i];
    }
    total
}

fn one_past(batch: &RefCell<Vec<u32>>) -> u32 {
    let mut total = 0;
    for i in (0..batch.borrow().len()).step_by(2) {
        total += batch.borrow()[i + 1];
    }
    total
}

fn grown_in_the_loop(batch: &RefCell<Vec<u32>>) -> u32 {
    let mut total = 0;
    for i in (0..batch.borrow().len()).step_by(2) {
        total += batch.borrow()[i];
        batch.borrow_mut().push(1);
    }
    total
}

fn run(name: &str, row: impl FnOnce() -> u32) {
    match panic::catch_unwind(AssertUnwindSafe(row)) {
        Ok(value) => println!("{name}: {value}"),
        Err(_) => println!("{name}: panicked"),
    }
}

fn main() {
    let batch = RefCell::new(vec![1, 2, 3, 4, 5]);
    run("stepped", || stepped(&batch));
    run("held", || held(&batch));
    run("each", || each(&batch));
    run("enumerated", || enumerated(&batch));
    run("held grown", || held_grown(&RefCell::new(vec![1, 2, 3])));
    run("held cleared", || held_cleared(&RefCell::new(vec![1, 2, 3])));
    run("held emptied by a call", || held_emptied_by_a_call(&RefCell::new(vec![1, 2, 3])));
    run("past the end, odd", || past_the_end(&RefCell::new(vec![1, 2, 3])));
    run("past the end, even", || past_the_end(&RefCell::new(vec![1, 2])));
    run("one past, even", || one_past(&RefCell::new(vec![1, 2, 3, 4])));
    run("one past, odd", || one_past(&RefCell::new(vec![1, 2, 3])));
    run("grown in the loop", || grown_in_the_loop(&RefCell::new(vec![1, 2])));
    println!("{:?}", batch.borrow());
}
