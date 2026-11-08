//! A library's impls whose methods its JS reaches only through their
//! dictionaries: an own `max`, an own `ne` between two types, and a trait's
//! generic default that clones what its bounds say is a `Vec` (#1023).
use std::cmp::Ordering;

#[derive(Debug, PartialEq, Eq)]
pub struct Picky(pub u32, pub char);

impl PartialOrd for Picky {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Picky {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.cmp(&other.0)
    }

    fn max(self, other: Self) -> Self {
        println!("own max");
        if other.0 > self.0 { other } else { self }
    }
}

pub struct A(pub u32);
pub struct B(pub u32);

impl PartialEq<B> for A {
    fn eq(&self, other: &B) -> bool {
        println!("eq {} {}", self.0, other.0);
        self.0 == other.0
    }

    #[allow(clippy::partialeq_ne_impl)]
    fn ne(&self, other: &B) -> bool {
        println!("ne {} {}", self.0, other.0);
        self.0 != other.0
    }
}

#[derive(Clone)]
pub struct Holder<T: Iterator> {
    pub item: T::Item,
}

pub trait Tr {
    fn copied<T: Iterator<Item = Vec<u32>> + Clone>(h: &Holder<T>) -> Holder<T> {
        h.clone()
    }
}

#[derive(Clone)]
pub struct Lists;

impl Iterator for Lists {
    type Item = Vec<u32>;

    fn next(&mut self) -> Option<Vec<u32>> {
        None
    }
}

pub struct S;

impl Tr for S {}

/// A writer whose own `write_str` fails, which the program compiles
/// itself, its `&mut self` unboxed, and whose failure its manifest lists.
pub struct Full(pub usize);

impl std::fmt::Write for Full {
    fn write_str(&mut self, s: &str) -> std::fmt::Result {
        if s.len() > self.0 {
            return Err(std::fmt::Error);
        }
        self.0 -= s.len();
        Ok(())
    }
}
