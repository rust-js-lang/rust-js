//! Box places a consumer writes through, of impls it compiles itself from
//! its library's optimized MIR (ADR 0914): a `&mut` to what a box holds,
//! which that MIR reads through a copy of the box, `_b = copy (*_r)`, is the
//! box's own place, as the crate's own MIR has it: a box's number, a box of
//! a box's, an enum variant's box's, a boxed struct's fields, a `Vec` of
//! boxes'. A write through one was lost (#1455).
use std::borrow::{Borrow, BorrowMut};
use std::ops::{Deref, DerefMut, Index, IndexMut};
pub struct Nested(pub Box<Box<u32>>);
impl AsMut<u32> for Nested {
    fn as_mut(&mut self) -> &mut u32 {
        &mut self.0
    }
}
pub enum Slot {
    Full(Box<u32>),
    Empty(u32),
}
impl Deref for Slot {
    type Target = u32;
    fn deref(&self) -> &u32 {
        match self {
            Slot::Full(b) => b,
            Slot::Empty(n) => n,
        }
    }
}
impl DerefMut for Slot {
    fn deref_mut(&mut self) -> &mut u32 {
        match self {
            Slot::Full(b) => &mut **b,
            Slot::Empty(n) => n,
        }
    }
}
pub struct Point {
    pub x: u32,
    pub y: u32,
}
pub struct Holder {
    pub point: Box<Point>,
    pub more: Vec<Box<u32>>,
}
impl Index<usize> for Holder {
    type Output = u32;
    fn index(&self, i: usize) -> &u32 {
        &self.more[i]
    }
}
impl IndexMut<usize> for Holder {
    fn index_mut(&mut self, i: usize) -> &mut u32 {
        match i {
            0 => &mut self.point.x,
            1 => &mut self.point.y,
            _ => &mut self.more[i - 2],
        }
    }
}
impl Borrow<Point> for Holder {
    fn borrow(&self) -> &Point {
        &self.point
    }
}
impl BorrowMut<Point> for Holder {
    fn borrow_mut(&mut self) -> &mut Point {
        &mut self.point
    }
}
pub struct Local;
impl Iterator for Local {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
        let mut b = Box::new(3u32);
        let r = &mut *b;
        *r += 1;
        *b += 10;
        Some(*b)
    }
}

/// A box of a number behind `DerefMut`.
pub struct Counter(pub Box<u32>);
impl Deref for Counter {
    type Target = u32;
    fn deref(&self) -> &u32 {
        &self.0
    }
}
impl DerefMut for Counter {
    fn deref_mut(&mut self) -> &mut u32 {
        &mut self.0
    }
}
