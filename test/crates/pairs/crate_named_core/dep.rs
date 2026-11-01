//@ crate-name: core
//! A program's crate named `core`, a workspace's (#455): an ordinary
//! crate, whose items rust-js never takes for std's, though each is named
//! as one of std's is, and does something else (ADR 1052).
use std::fmt;

pub fn double(x: i32) -> i32 {
    x * 2
}

pub struct Point {
    pub x: i32,
    pub y: i32,
}

impl fmt::Display for Point {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({}, {})", self.x, self.y)
    }
}

/// `std::mem::take`'s name: it leaves what it's given as it is.
pub mod mem {
    pub fn take(v: &mut Vec<i32>) -> usize {
        v.len()
    }
}

/// `std::hint::black_box`'s name: it's not what it's given.
pub mod hint {
    pub fn black_box(x: i32) -> i32 {
        x + 1
    }
}

/// `std::iter::once`'s name: an item twice.
pub mod iter {
    pub fn once(x: i32) -> Vec<i32> {
        vec![x, x]
    }
}

/// `std::any::type_name`'s name: a name of its own.
pub mod any {
    pub fn type_name<T>() -> &'static str {
        "mine"
    }
}

/// `char::from_digit`'s name: a digit of its own.
pub mod char {
    pub fn from_digit(digit: u32, _radix: u32) -> Option<char> {
        (digit == 7).then_some('x')
    }
}

/// `Option`'s path, `core::option::Option`, of a type of its own: never
/// std's, by a `TypeId` either.
pub mod option {
    pub struct Option<T>(pub T);
}

pub fn boxed() -> Box<dyn std::any::Any> {
    Box::new(option::Option(1))
}

pub fn is_std_option(value: &dyn std::any::Any) -> bool {
    value.downcast_ref::<std::option::Option<i32>>().is_some()
}
