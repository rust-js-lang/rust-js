//! A library that compares `Rc`s of a `dyn Fn` by identity: its consumers'
//! `Rc`s, which it doesn't see made, may be made of one function item, so
//! it counts them, each compared by its box (ADR 0959).
use std::rc::Rc;

pub fn same<T>(a: &Rc<dyn Fn(T)>, b: &Rc<dyn Fn(T)>) -> bool {
    Rc::ptr_eq(a, b)
}

pub fn hook<T>() -> Rc<dyn Fn(T)> {
    Rc::new(|_| {})
}
