//@ compile-fail: `std::borrow::Cow::<'_, B>::into_owned` cloning the `RefCell`s a `[std::cell::RefCell<u8>]` holds, as where std's code of it raises a borrowed cell's clone isn't known (#2627)
// A `Cow<[RefCell<_>]>`'s `into_owned` clones the cells its slice holds, as
// std's `to_vec` does; the walk can't make `B` of a cell there, so where a
// borrowed one's panic is raised isn't known, and rust-js refuses it rather
// than name the caller's line. `cow.to_vec()` is the same clone, whose
// line it knows (#2627).
use std::borrow::Cow;
use std::cell::RefCell;

fn main() {
    let items = vec![RefCell::new(1u8)];
    let cow: Cow<[RefCell<u8>]> = Cow::Borrowed(&items[..]);
    println!("{:?}", cow.into_owned());
}
