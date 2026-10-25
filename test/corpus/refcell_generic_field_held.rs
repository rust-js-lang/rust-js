//@ run-fail: RefCell already borrowed
// A generic struct's `RefCell<T>` field holds its value; made of an array
// and borrowed in its generic code while code that may borrow it runs, the
// `RefCell<Vec<u8>>` its concrete code borrows is a box of it too, so both
// count on the field's one cell, and the mutable borrow panics as std's
// does (ADR 0863, amended).
use std::cell::RefCell;

struct Cursor<T> {
    current: RefCell<T>,
}

fn after<T: Clone>(value: &T, f: impl Fn()) -> T {
    f();
    value.clone()
}

fn peek<T: Clone>(cursor: &Cursor<T>, f: impl Fn()) -> T {
    after(&*cursor.current.borrow(), f)
}

fn main() {
    let cursor = Cursor {
        current: RefCell::new(vec![1u8]),
    };
    println!("{:?}", peek(&cursor, || {}));
    let seen = peek(&cursor, || cursor.current.borrow_mut().push(2));
    println!("{seen:?}");
}
