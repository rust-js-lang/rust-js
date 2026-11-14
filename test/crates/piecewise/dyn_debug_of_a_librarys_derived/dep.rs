//! A library that makes a `dyn Debug` of a published type of its own whose
//! `Debug` is derived, which a consumer compiles its own copy of: the
//! library's dictionary carries no `fmtInto`, so its contract says it makes
//! a `dyn` that may carry none (#1688).
use std::fmt;

#[derive(Debug)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

pub fn shown() -> Box<dyn fmt::Debug> {
    Box::new(Point { x: 1, y: -2 })
}
