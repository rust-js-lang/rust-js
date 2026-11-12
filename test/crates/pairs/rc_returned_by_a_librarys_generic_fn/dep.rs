//! A library's generic function that makes and returns an `Rc<T>`, and one
//! that holds a clone of it while it reads its count, none called by its own
//! code (#1234).
use std::rc::Rc;

pub fn make<T>(value: T) -> Rc<T> {
    Rc::new(value)
}

pub fn shared<T>(r: &Rc<T>) -> (Rc<T>, Rc<T>) {
    (Rc::clone(r), Rc::clone(r))
}
