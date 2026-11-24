//@ compile-fail: a value of `RefCell::clone`, a `#[track_caller]` function, called where this function doesn't say how (#2335)
// `RefCell::clone` returned by a `pub` function, which JS or another crate
// may call and call what it returns as it likes: where its panic is raised
// isn't told by the crate's own callers, so a binary refuses it, as #2335
// refuses a value whose calls it doesn't see (#2530).
use std::cell::RefCell;

pub fn copier() -> impl Fn(&RefCell<u8>) -> RefCell<u8> {
    RefCell::clone
}

fn main() {
    let items = vec![RefCell::new(1u8)];
    let copy = copier();
    println!("{:?}", copy(&items[0]));
}
