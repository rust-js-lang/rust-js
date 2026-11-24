//@ run-fail: RefCell already mutably borrowed
// `RefCell::clone` returned as an `impl Fn`, called by the caller through
// `Fn::call`: its clone of a `RefCell` mutably borrowed panics at that
// shim's line, `function.rs:79:5`, as the function's one caller uses what
// it returns (#2530).
use std::cell::RefCell;

fn copier() -> impl Fn(&RefCell<u8>) -> RefCell<u8> {
    RefCell::clone
}

fn main() {
    let items = vec![RefCell::new(1u8), RefCell::new(2)];
    let _held = items[1].borrow_mut();
    let copy = copier();
    println!("{:?}", copy(&items[0]));
    println!("{:?}", copy(&items[1]));
}
