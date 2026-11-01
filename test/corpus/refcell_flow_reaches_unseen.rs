//@ run-fail: RefCell already mutably borrowed
// A cell given to what may call anything, a closure's generic caller
// whose `f()` may be any function, may be any cell of its type that code
// reaches (ADR 0863, amended): a borrow held across it is checked, and one
// taken through what the closure captured panics, as std's does.

use std::cell::RefCell;

fn twice(f: impl Fn()) {
    f();
    f();
}

fn main() {
    let cells = vec![RefCell::new(vec![1]), RefCell::new(vec![2])];
    let picked = Some(&cells).map(|cells| &cells[0]);
    let held = cells[0].borrow_mut();
    println!("{}", held.len());
    twice(|| {
        if let Some(cell) = picked {
            println!("{}", cell.borrow().len());
        }
    });
    println!("{}", held.len());
}
