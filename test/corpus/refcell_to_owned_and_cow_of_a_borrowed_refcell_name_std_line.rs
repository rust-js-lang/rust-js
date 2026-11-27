//@ run-fail: RefCell already mutably borrowed
// A `RefCell`'s `to_owned()`, a `Cow<RefCell<_>>`'s `to_mut()` and its
// `into_owned()` of one mutably borrowed each panic where std's blanket
// `ToOwned` clones it, `borrow.rs:78:14`, read from std's MIR of every std
// function a call is resolved to (`clonedAt`, #2509), not the caller's.
use std::borrow::Cow;
use std::cell::RefCell;
use std::panic::{self, AssertUnwindSafe};

fn main() {
    let cell = RefCell::new(1u8);
    let mut cow: Cow<RefCell<u8>> = Cow::Borrowed(&cell);
    *cow.to_mut().get_mut() += 1;
    println!("{:?} {:?} {cow:?}", cell.to_owned(), Cow::Borrowed(&cell).into_owned());
    let _held = cell.borrow_mut();
    let owned = panic::catch_unwind(AssertUnwindSafe(|| cell.to_owned().into_inner()));
    println!("{}", owned.is_err());
    let made_mut = panic::catch_unwind(AssertUnwindSafe(|| {
        let mut cow: Cow<RefCell<u8>> = Cow::Borrowed(&cell);
        cow.to_mut().get_mut().to_string()
    }));
    println!("{}", made_mut.is_err());
    println!("{:?}", Cow::Borrowed(&cell).into_owned());
}
