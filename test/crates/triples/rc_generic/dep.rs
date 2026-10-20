// A generic library that makes and gives back an `Rc<T>`, counts it, and
// changes it in place, `make_mut` and `get_mut` (ADR 0523).
use std::rc::Rc;

pub fn share<T>(t: T) -> Rc<T> {
    Rc::new(t)
}

pub fn owners<T>(rc: &Rc<T>) -> usize {
    Rc::strong_count(rc)
}

pub fn bump(rc: &mut Rc<Vec<u32>>) {
    Rc::make_mut(rc).push(9);
}

pub fn set_if_alone(rc: &mut Rc<u32>, value: u32) -> bool {
    match Rc::get_mut(rc) {
        Some(x) => {
            *x = value;
            true
        }
        None => false,
    }
}
