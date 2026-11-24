//@ run-fail: RefCell already mutably borrowed
// An `Option<&RefCell<_>>`'s `cloned()` of one mutably borrowed panics
// where std's `map(T::clone)` clones it, through `FnOnce::call_once`'s
// shim, `function.rs:250:5`, never `Option`'s own clone's line
// (`clonedAt`, #2509).
use std::cell::RefCell;

fn main() {
    let items = vec![RefCell::new(1u8), RefCell::new(2)];
    let _held = items[1].borrow_mut();
    println!("{:?}", items.first().cloned());
    println!("{:?}", items.last().cloned());
}
