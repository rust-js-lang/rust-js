//! A `Copy` type this crate never changes in place, a constant of it Rust
//! makes afresh at each use, and one a cell it hands out an `Rc` of holds:
//! each read of what it keeps is a copy of its own (#1907).
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Clone, Copy, Debug)]
pub struct Spot {
    pub x: i32,
    pub y: i32,
}

pub const ORIGIN: Spot = Spot { x: 0, y: 0 };

pub fn origin() -> Spot {
    ORIGIN
}

pub fn store() -> Rc<RefCell<Spot>> {
    Rc::new(RefCell::new(ORIGIN))
}

pub fn read(store: &Rc<RefCell<Spot>>) -> Spot {
    *store.borrow()
}

/// What it stores, of which only a number leaves.
pub fn origin_x() -> i32 {
    let origin = ORIGIN;
    origin.x
}

thread_local! {
    static HOME: Mark = const { Mark { n: 5 } };
}

pub fn home() -> Mark {
    HOME.with(|home| *home)
}

#[derive(Clone, Copy, Debug)]
pub struct Pair {
    pub first: Mark,
    pub second: Mark,
}

pub const PAIR: Pair = Pair {
    first: Mark { n: 0 },
    second: Mark { n: 1 },
};

pub fn pair() -> Pair {
    PAIR
}

/// A `Copy` type no cell of this crate's holds, so nothing of this crate
/// changes one: of what an `Rc` its consumer holds points at, only its
/// being stored copies what it hands out.
#[derive(Clone, Copy, Debug)]
pub struct Mark {
    pub n: i32,
}

pub fn shared() -> Rc<Mark> {
    Rc::new(Mark { n: 1 })
}

pub fn got(shared: &Rc<Mark>) -> Mark {
    **shared
}

/// Of what it stores, only a number leaves.
pub fn got_n(shared: &Rc<Mark>) -> i32 {
    let mark = **shared;
    mark.n
}
