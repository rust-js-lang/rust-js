//@ run-fail: RefCell already mutably borrowed
// A guard a `let` holds that's moved out of its block outlives it, so it
// may meet a borrow after it: its cell's borrows are counted (ADR 0863),
// and the borrow it meets panics, as std's does.

use std::cell::{RefCell, RefMut};

fn main() {
    let cell = RefCell::new(vec![1]);
    let kept: RefMut<'_, Vec<i32>>;
    {
        let v = cell.borrow_mut();
        kept = v;
    }
    println!("{}", kept.len());
    println!("{}", cell.borrow().len());
}
