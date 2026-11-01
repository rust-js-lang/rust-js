//! A crate that sees no crate named `core` but std's, making `dyn Any`s of
//! std's types, whose `TypeId`s are the same in a crate that sees one
//! (#455, ADR 1054).
use std::any::Any;

pub fn some() -> Box<dyn Any> {
    Box::new(Some(1))
}

pub fn text() -> Box<dyn Any> {
    Box::new(String::from("text"))
}
