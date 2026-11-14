//! A library that makes a `dyn Debug` of a std value whose `Debug` isn't
//! followed a piece at a time: its dictionary is left without `fmtInto`,
//! never refused, which its lowering gives back into its contract
//! (`makes_fmt_dyns`, #1797).
use std::fmt;
use std::time::Duration;

pub fn shown() -> Box<dyn fmt::Debug> {
    Box::new(Duration::from_millis(1500))
}
