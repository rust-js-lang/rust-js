//! A library that makes a `dyn Debug` of a std value, whose pair's
//! dictionary carries `fmtInto` best-effort, as a consumer may write it a
//! piece at a time (#1797).
use std::fmt;

pub fn shown() -> Box<dyn fmt::Debug> {
    Box::new(Some(7))
}
