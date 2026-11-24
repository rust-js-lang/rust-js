//@ run-fail: RefCell already mutably borrowed
// Generic code's `T::clone` returned as an `impl Fn`, of a `RefCell`, whose
// impl tracks its caller: called through `Fn::call` by the function's
// caller, it panics at that shim's line, `function.rs:79:5` (#2530).
use std::cell::RefCell;

fn copier<T: Clone>() -> impl Fn(&T) -> T {
    T::clone
}

fn main() {
    let numbers = copier::<u8>();
    println!("{}", numbers(&3));
    let items = vec![RefCell::new(1u8), RefCell::new(2)];
    let _held = items[1].borrow_mut();
    let copy = copier::<RefCell<u8>>();
    println!("{:?}", copy(&items[0]));
    println!("{:?}", copy(&items[1]));
}
