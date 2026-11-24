//@ run-fail: RefCell already mutably borrowed
// A held `cloned()` stepped by `next()` past a first item to a `RefCell`
// mutably borrowed panics where std's `Cloned::next` clones it, through
// `FnOnce::call_once`'s shim, `function.rs:250:5` (`clonedAt`, #2509).
use std::cell::RefCell;

fn main() {
    let items = vec![RefCell::new(1u8), RefCell::new(2)];
    let _held = items[1].borrow_mut();
    let mut each = items.iter().cloned();
    println!("{:?}", each.next());
    println!("{:?}", each.next());
}
