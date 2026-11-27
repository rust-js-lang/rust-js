//@ compile-fail: `std::slice::<impl [T]>::join` cloning the `RefCell`s a `std::vec::Vec<std::cell::RefCell<u8>>` holds, as where std's code of it raises a borrowed cell's clone isn't known (#2627)
// A slice's `join` of `Vec`s of `RefCell`s clones each part's cells and the
// separator's, at two lines of std's, which the walk can't tell apart, so
// rust-js refuses it rather than name the caller's line; a `Vec` extended
// by `extend_from_slice` of each is the same, whose line it knows (#2627).
use std::cell::RefCell;

fn main() {
    let parts = vec![vec![RefCell::new(1u8)], vec![RefCell::new(2)]];
    let separator = RefCell::new(0u8);
    println!("{:?}", parts.join(&separator));
}
