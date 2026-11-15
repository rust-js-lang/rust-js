//! `Copy` types this crate never changes in place: what it hands out is
//! shared with its consumers', as nothing of its own changes them.

#[derive(Clone, Copy, Debug)]
pub struct Spot {
    pub x: i32,
    pub y: i32,
}

pub fn spot(x: i32, y: i32) -> Spot {
    Spot { x, y }
}

/// Every spot it's given, as it was given.
pub fn spots(first: Spot) -> [Spot; 2] {
    [first, Spot { x: first.y, y: first.x }]
}
