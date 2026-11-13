// A `RefCell` borrowed for a moment in a loop's test, a short circuit's
// right side or a conditional's branch is checked each time it's
// evaluated there, and only then: its loop is the one written, `for` or
// `while (cond)`, never `while (true)` and a `break`, as React's
// flushPreamble and fulfillReference write theirs (ADR 1141, amended).
// One evaluated first in its statement is checked before it.
use std::cell::RefCell;

fn summed(cell: &RefCell<Vec<u32>>) -> u32 {
    let mut total = 0;
    let mut i = 0;
    while i < cell.borrow().len() {
        total += cell.borrow()[i];
        i += 1;
    }
    total
}

fn drained(cell: &RefCell<Vec<u32>>) -> u32 {
    let mut total = 0;
    while !cell.borrow().is_empty() {
        total += cell.borrow_mut().remove(0);
    }
    total
}

fn guarded(cell: &RefCell<Vec<u32>>, look: bool) -> bool {
    look && !cell.borrow().is_empty()
}

fn chosen(cell: &RefCell<Vec<u32>>, look: bool) -> usize {
    if look { cell.borrow().len() } else { 0 }
}

fn main() {
    let cell = RefCell::new(vec![1, 2, 3]);
    println!("{}", summed(&cell));
    let held = cell.borrow_mut();
    println!("{} {}", guarded(&cell, false), chosen(&cell, false));
    drop(held);
    println!("{} {}", guarded(&cell, true), chosen(&cell, true));
    println!("{} {:?}", drained(&cell), cell.borrow());
}
