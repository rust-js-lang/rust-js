//! A library that makes a `dyn Debug` of a value of a type parameter, which
//! a consumer's value may stand for, whose dictionary it can't say carries
//! `fmtInto`: its contract says it may make one that carries none
//! (`makes_fmt_dyns`, #1797).
use std::fmt;

pub fn boxed<T: fmt::Debug + 'static>(t: T) -> Box<dyn fmt::Debug> {
    Box::new(t)
}
