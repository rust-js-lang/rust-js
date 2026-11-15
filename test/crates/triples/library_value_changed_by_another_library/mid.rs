//! Changes `dep`'s `Spot` in place, which `dep` never does, by each route
//! a value reaches it: by value, through a `&mut`, a `RefCell` given by
//! `&`, a trait's method, what a callback gives back, and a `&mut` inside
//! a struct (#1907).
use dep::Spot;
use std::cell::RefCell;

pub fn nudged(mut spot: Spot) -> Spot {
    spot.x += 1;
    spot
}

pub fn shift(spot: &mut Spot) {
    spot.y += 10;
}

pub struct Cells {
    pub spot: RefCell<Spot>,
}

pub fn bump(cells: &Cells) {
    cells.spot.borrow_mut().x += 100;
}

pub trait Mover {
    fn moved(&self, spot: &mut Spot);
}

pub struct Up;

impl Mover for Up {
    fn moved(&self, spot: &mut Spot) {
        spot.y -= 1000;
    }
}

pub fn with_spot(made: impl FnOnce() -> Spot) -> Spot {
    let mut spot = made();
    spot.y += 3;
    spot
}

pub struct Target<'a> {
    pub spot: &'a mut Spot,
}

pub fn aim(target: Target) {
    target.spot.x += 7;
}

pub fn shift_first(pair: &mut dep::Pair) {
    pair.first.n += 20;
}

pub fn marked(mark: &mut dep::Mark) {
    mark.n += 100;
}
