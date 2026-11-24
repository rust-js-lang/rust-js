//@ run-fail: RefCell already mutably borrowed
// `RefCell::clone` given as an `F: FnMut` that the callee moves into
// `Option::map`, which calls it through `FnOnce::call_once`: it panics at
// that shim's line, `function.rs:250:5`, as the callee's MIR says, not the
// `call_mut` its bound alone would say (#2522).
use std::cell::RefCell;

fn first_copy<T, F: FnMut(&T) -> T>(f: F, items: &[T]) -> Option<T> {
    items.first().map(f)
}

fn main() {
    let items = vec![RefCell::new(1u8)];
    let _held = items[0].borrow_mut();
    println!("{}", items.len());
    println!("{:?}", first_copy(RefCell::clone, &items));
}
