//@ run-fail: RefCell already mutably borrowed
// A `Result<&RefCell<_>, E>`'s `cloned()` of one mutably borrowed panics
// where std's `map(|t| t.clone())` clones it, `result.rs:1744:24`, and a
// `Result<&mut RefCell<_>, E>`'s at its own, `1795:24`, read from std's MIR
// of every std function a call is resolved to (`clonedAt`, #2509).
use std::cell::RefCell;
use std::panic::{self, AssertUnwindSafe};

fn main() {
    let items = vec![RefCell::new(1u8), RefCell::new(2)];
    let found: Result<&RefCell<u8>, ()> = items.first().ok_or(());
    println!("{:?}", found.cloned());
    let mut cell = RefCell::new(3u8);
    std::mem::forget(cell.borrow_mut());
    let lent: Result<&mut RefCell<u8>, ()> = Ok(&mut cell);
    let mut_cloned = panic::catch_unwind(AssertUnwindSafe(|| lent.cloned().is_ok()));
    println!("{}", mut_cloned.is_err());
    let _held = items[1].borrow_mut();
    let found: Result<&RefCell<u8>, ()> = items.last().ok_or(());
    println!("{:?}", found.cloned());
}
