//! A library's functions that show a `&dyn Debug` they're given with a
//! `Formatter`'s options, `{:#?}`, `{:x?}` and `{:#x?}`, and one that holds
//! them first: a `dyn Debug` is its value with its type's `Debug`, `{ value,
//! impl }`, whose writer its consumer makes take options, as the type is
//! made a `dyn Debug` (#1335).
use std::fmt::Debug;

pub fn pretty(d: &dyn Debug) -> String {
    format!("{d:#?}")
}

pub fn hex(d: &dyn Debug) -> String {
    format!("{d:x?}")
}

pub fn pretty_hex(d: &dyn Debug) -> String {
    format!("{d:#x?}")
}

pub fn all_pretty(all: Vec<Box<dyn Debug>>) -> String {
    format!("{all:#?}")
}
