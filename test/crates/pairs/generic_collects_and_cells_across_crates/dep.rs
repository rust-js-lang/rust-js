//! A library's generic code that collects in place, borrows a `RefCell` a
//! moment, makes a fresh array, and shows with a placeholder's options,
//! compiled by the app that gives it its types (ADR 0914). Decision
//! coverage found each given only in a crate's own bodies (#1376's second
//! pass).
use std::cell::RefCell;
use std::fmt::Display;
use std::ops::Add;

pub fn doubled<T: Copy + Add<Output = T>>(v: Vec<T>) -> Vec<T> {
    v.into_iter().map(|x| x + x).collect()
}

pub fn at_least<T: PartialOrd>(v: Vec<T>, min: T) -> Vec<T> {
    v.into_iter().filter(|x| *x >= min).collect()
}

pub fn read<T: Clone>(cell: &RefCell<T>) -> T {
    cell.borrow().clone()
}

pub fn len_of<T>(cell: &RefCell<Vec<T>>) -> usize {
    cell.borrow().len()
}

pub fn push_twice<T: Clone>(cell: &RefCell<Vec<T>>, item: T) -> usize {
    cell.borrow_mut().push(item.clone());
    cell.borrow_mut().push(item);
    cell.borrow().len()
}

pub fn three<T: Copy>(x: T) -> [T; 3] {
    [x; 3]
}

pub fn padded<T: Display>(x: T) -> String {
    format!("[{x:>8}] [{x:<6}] [{x:^7}]")
}
