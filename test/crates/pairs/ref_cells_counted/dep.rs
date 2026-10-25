//! A library whose generic code has a `RefCell<T>` of any `T`, so each of
//! its `RefCell`s of an object is a `{ value }`, and which holds a borrow
//! of a `RefCell<Vec<i32>>` while a function it's given runs, so each is
//! counted (ADR 0863): its consumer boxes and checks them as it does.
use std::cell::RefCell;
use std::rc::Rc;

pub fn shared<T>(value: T) -> Rc<RefCell<T>> {
    Rc::new(RefCell::new(value))
}

pub fn each(list: &RefCell<Vec<i32>>, f: impl Fn(i32)) {
    for &n in list.borrow().iter() {
        f(n);
    }
}
