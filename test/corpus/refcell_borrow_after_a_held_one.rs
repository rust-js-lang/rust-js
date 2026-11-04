//@ run-fail: RefCell already mutably borrowed
// A borrow made after another its statement still holds meets it, and
// panics, as std's does (#552): what ran before it was made is skipped only
// where it's ended, and the `RefMut` `format_args!` made first lives to the
// statement's end.

use std::cell::RefCell;

fn main() {
    let log = RefCell::new(vec![1]);
    println!("{:?} {}", log.borrow_mut(), log.borrow().len());
}
