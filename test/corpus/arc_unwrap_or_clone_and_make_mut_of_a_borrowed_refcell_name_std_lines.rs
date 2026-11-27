//@ run-fail: RefCell already mutably borrowed
// An `Arc`'s `unwrap_or_clone` and `make_mut` of a `RefCell` mutably
// borrowed, shared, each panic where std's clones it, `sync.rs:2759:59`
// and `clone/uninit.rs:20:33`, read from std's MIR (`clonedAt`, #2509).
use std::cell::RefCell;
use std::panic::{self, AssertUnwindSafe};
use std::sync::Arc;

fn main() {
    let shared = Arc::new(RefCell::new(1u8));
    let other = Arc::clone(&shared);
    let _held = other.borrow_mut();
    let unwrapped = panic::catch_unwind(AssertUnwindSafe(|| Arc::unwrap_or_clone(shared)));
    println!("{}", unwrapped.is_err());
    let mut again = Arc::clone(&other);
    println!("{:?}", Arc::make_mut(&mut again).try_borrow().is_ok());
}
