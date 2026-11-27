//@ run-fail: RefCell already mutably borrowed
// A slice's `to_owned()` and `Vec::from(&[_])` of a `RefCell` mutably
// borrowed each panic where std's `to_vec` clones it, `slice.rs:432:38`,
// read from std's MIR of the function rustc resolves each call to, `<[T]
// as ToOwned>::to_owned` and `<Vec<T> as From<&[T]>>::from` (`clonedAt`,
// #2509).
use std::cell::RefCell;
use std::panic::{self, AssertUnwindSafe};

fn main() {
    let items = vec![RefCell::new(1u8), RefCell::new(2)];
    println!("{:?} {:?}", items[..].to_owned(), Vec::from(&items[..]));
    let _held = items[1].borrow_mut();
    let owned = panic::catch_unwind(AssertUnwindSafe(|| items[..].to_owned().len()));
    println!("{}", owned.is_err());
    let from: Vec<RefCell<u8>> = Vec::from(&items[..]);
    println!("{}", from.len());
}
