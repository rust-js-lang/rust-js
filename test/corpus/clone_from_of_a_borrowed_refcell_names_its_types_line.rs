//@ run-fail: RefCell already mutably borrowed
// `clone_from` of an array, a `Box`, a `Result` and a `Reverse` of a
// `RefCell` mutably borrowed panics where each std `clone_from` calls the
// cell's, `array/mod.rs:472:14`, `boxed.rs:2189:18`, `result.rs:1906:38`
// and `cmp.rs:745:16`, its `clonedFromAt` row (#2627).
use std::cell::RefCell;
use std::cmp::Reverse;
use std::panic::{self, AssertUnwindSafe};

fn main() {
    let items = [RefCell::new(1u8), RefCell::new(2)];
    let _held = items[1].borrow_mut();
    let mut array = [RefCell::new(5u8), RefCell::new(6)];
    let written = panic::catch_unwind(AssertUnwindSafe(|| array.clone_from(&items)));
    println!("{} {:?}", written.is_err(), array.iter().map(|cell| *cell.borrow()).collect::<Vec<_>>());
    let boxed = Box::new(RefCell::new(3u8));
    let _boxed_held = boxed.borrow_mut();
    let mut other = Box::new(RefCell::new(0u8));
    println!("{}", panic::catch_unwind(AssertUnwindSafe(|| other.clone_from(&boxed))).is_err());
    let ok: Result<RefCell<u8>, ()> = Ok(RefCell::new(4));
    let _ok_held = ok.as_ref().unwrap().borrow_mut();
    let mut into: Result<RefCell<u8>, ()> = Ok(RefCell::new(0));
    println!("{}", panic::catch_unwind(AssertUnwindSafe(|| into.clone_from(&ok))).is_err());
    let reversed = Reverse(RefCell::new(5u8));
    let _reversed_held = reversed.0.borrow_mut();
    let mut back = Reverse(RefCell::new(0u8));
    back.clone_from(&reversed);
}
