//! A library that writes `{:#?}` and asks no `Formatter`'s option itself,
//! whose writers forward by path what a consumer gives them: each still
//! applies a width, a fill or a `+` a consumer gives, as its manifest says
//! its writers take options (#1164).
use std::fmt;

#[derive(Debug)]
pub struct Point {
    pub x: u32,
    pub y: i32,
}

/// Its `Debug` its `Display`, and its `Display` its number's.
pub struct Count(pub u32);

impl fmt::Display for Count {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

impl fmt::Debug for Count {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

/// Its `Debug` by std's builders, each field shown with the options given.
pub struct Span(pub u32, pub i64);

impl fmt::Debug for Span {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Span").field("from", &self.0).field("to", &self.1).finish()
    }
}

/// A `Point` its consumer shows by the library's own writer, a `dyn`'s.
pub fn boxed(point: Point) -> Box<dyn fmt::Debug> {
    Box::new(point)
}

pub fn pretty(point: &Point) -> String {
    format!("{point:#?}")
}
