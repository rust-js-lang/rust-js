// A library's function that tracks its caller and reads where it was called
// takes it, as its manifest says, `takes_location`, which its consumer's
// calls give (ADR 0427); one that reads nothing of it takes none.
use std::panic::Location;

#[track_caller]
pub fn line_here() -> u32 {
    Location::caller().line()
}

#[track_caller]
pub fn column_here() -> u32 {
    Location::caller().column()
}

#[track_caller]
pub fn quiet(x: u32) -> u32 {
    x + 1
}

// A trait whose tracked default reads where it was called: each entry of it
// takes the location, as the manifest says of the default, which its
// consumer's calls through a dictionary give, and its impls take.
pub trait Tracked {
    #[track_caller]
    fn line(&self) -> u32 {
        Location::caller().line()
    }
}

pub struct Here;

impl Tracked for Here {}

pub fn lines() -> Vec<u32> {
    let items: [&dyn Tracked; 1] = [&Here];
    items.iter().map(|item| item.line()).collect()
}

// A trait's tracked method without a default: each entry of it takes the
// location in every crate alike, as its declaration tracks its caller,
// which its consumer sees as this crate does, its impl here that reads it
// and its impl there that doesn't, through generic code of either (#2350).
pub trait Checked {
    #[track_caller]
    fn checked(&self, n: u32) -> u32;
}

pub struct Strict;

impl Checked for Strict {
    fn checked(&self, n: u32) -> u32 {
        Location::caller().line() * 10 + n
    }
}

pub fn check_one<C: Checked>(c: &C) -> u32 {
    c.checked(1)
}
