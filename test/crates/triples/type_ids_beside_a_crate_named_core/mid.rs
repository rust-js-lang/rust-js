//@ crate-name: core
//! A program's crate named `core` (#455), which sees std's beside it: a
//! `TypeId` of std's types is the one any crate has, and one of its own
//! types, whose path is std's `Option`'s, never theirs (ADR 1054).
use std::any::Any;

pub mod option {
    pub struct Option<T>(pub T);
}

pub fn which(value: &dyn Any) -> &'static str {
    if value.is::<Option<i32>>() {
        "std's Option"
    } else if value.is::<option::Option<i32>>() {
        "core's Option"
    } else if value.is::<String>() {
        "a String"
    } else {
        "neither"
    }
}

pub fn own() -> Box<dyn Any> {
    Box::new(option::Option(1))
}
