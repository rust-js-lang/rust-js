//! A library's generic functions that make concrete adapters of what
//! they're given and step them from their back, collect them in place,
//! borrow a `RefCell` a moment and make fresh arrays: each compiled by the
//! app that gives it a type that drops (ADR 0914), where decision coverage
//! found those answers given only in a crate's own bodies (#1376's second
//! pass).
use std::cell::RefCell;

pub fn reversed<T>(v: Vec<T>) -> Vec<T> {
    v.into_iter().rev().collect()
}

pub fn tail_pairs<T>(v: Vec<T>) -> Vec<(usize, T)> {
    v.into_iter().enumerate().rev().take(2).collect()
}

pub fn without<T: PartialEq>(v: Vec<T>, out: &T) -> Vec<T> {
    v.into_iter().filter(|x| x != out).collect()
}

pub fn first_last<T>(v: Vec<T>) -> (Option<T>, Option<T>) {
    let mut it = v.into_iter();
    let first = it.next();
    (first, it.next_back())
}

pub fn peek<T: Clone>(cell: &RefCell<Vec<T>>) -> Option<T> {
    cell.borrow().last().cloned()
}

pub fn pair_of<T: Clone>(x: T) -> [T; 2] {
    [x.clone(), x]
}

pub fn chained_back<T>(a: Vec<T>, b: Vec<T>) -> Vec<T> {
    a.into_iter().chain(b).rev().skip(1).collect()
}
