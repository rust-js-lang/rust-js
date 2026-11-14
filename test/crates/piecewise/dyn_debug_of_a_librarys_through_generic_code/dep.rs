//! A library that makes a `dyn Debug` of a value of its own, whose pair's
//! dictionary nothing here asks to write into a writer a piece at a time.
use std::fmt;

pub fn shown() -> Box<dyn fmt::Debug> {
    Box::new(Some(7))
}
