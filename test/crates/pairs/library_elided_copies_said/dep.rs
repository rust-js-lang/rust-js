//! A library's copies of what its own crate never changes in place, each
//! left out of its JS and said in its contract: a `Cow`'s `into_owned`, an
//! array held by value in a struct, and a `Copy` field through a getter.
use std::borrow::Cow;

#[derive(Clone, Copy, Debug)]
pub struct Spot {
    pub x: i32,
}

/// What `spots` holds, owned: `into_owned` clones each.
pub fn owned_spots(spots: &[Spot]) -> Vec<Spot> {
    Cow::Borrowed(spots).into_owned()
}

#[derive(Debug)]
pub struct Grid {
    pub cells: [i32; 3],
}

/// A grid of `grid`'s cells, its array copied by value.
pub fn grid_of(grid: &Grid) -> Grid {
    Grid { cells: grid.cells }
}

pub struct Holder {
    spot: Spot,
}

impl Holder {
    pub fn new(x: i32) -> Holder {
        Holder { spot: Spot { x } }
    }

    /// Its spot, a copy.
    pub fn spot(&self) -> Spot {
        self.spot
    }
}
