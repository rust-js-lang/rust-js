//! A library sharing a `RefCell` of an array as the array, as React's
//! `shared` does its internals' `thrownErrors` (ADR 0863).
use std::cell::RefCell;

pub struct Internals {
    pub thrown_errors: RefCell<Vec<Option<&'static str>>>,
}

pub fn thrown_errors(internals: &Internals) -> &RefCell<Vec<Option<&'static str>>> {
    &internals.thrown_errors
}
