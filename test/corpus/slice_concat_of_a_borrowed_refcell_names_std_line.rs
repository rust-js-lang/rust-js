//@ run-fail: RefCell already mutably borrowed
// A slice's `concat` of `Vec`s or arrays of `RefCell`s, one mutably
// borrowed, panics where std's `Concat` pushes a clone of each, by
// `extend_from_slice`, `function.rs:166:5`: the walk makes `concat`'s `V:
// Borrow<[T]>` a `Vec` of the cell where a `u8` holds no clause (#2627).
use std::cell::RefCell;
use std::panic::{self, AssertUnwindSafe};

fn main() {
    let parts = vec![vec![RefCell::new(1u8)], vec![RefCell::new(2), RefCell::new(3)]];
    println!("{:?}", parts.concat());
    let arrays = [[RefCell::new(4u8)], [RefCell::new(5)]];
    let _held = arrays[1][0].borrow_mut();
    let joined = panic::catch_unwind(AssertUnwindSafe(|| arrays.concat().len()));
    println!("{}", joined.is_err());
    let _held_too = parts[1][1].borrow_mut();
    println!("{}", parts.concat().len());
}
