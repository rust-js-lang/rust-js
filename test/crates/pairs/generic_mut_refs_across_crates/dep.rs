//! A library's generic code of `&mut T`, compiled by the app that gives it
//! its types (ADR 0914): written through, held in a struct, taken and
//! replaced, swapped within a slice. Decision coverage found its forms,
//! a handle, a box and a held box, given only in a crate's own bodies,
//! never in another's (#1376's second pass).
use std::ops::AddAssign;

pub fn bump<T: AddAssign + Copy>(x: &mut T, by: T) {
    *x += by;
}

pub fn swap_ends<T>(v: &mut [T]) {
    let n = v.len();
    if n > 1 {
        v.swap(0, n - 1);
    }
}

pub struct Slot<'a, T> {
    pub at: &'a mut T,
}

impl<T: Copy> Slot<'_, T> {
    pub fn set(&mut self, v: T) {
        *self.at = v;
    }

    pub fn get(&self) -> T {
        *self.at
    }
}

pub fn hold<T>(at: &mut T) -> Slot<'_, T> {
    Slot { at }
}

pub fn renew<T: Default, F: FnOnce(T) -> T>(x: &mut T, f: F) {
    let old = std::mem::take(x);
    *x = f(old);
}

pub fn both<A: AddAssign + Copy, B: AddAssign + Copy>(pair: &mut (A, B), a: A, b: B) {
    bump(&mut pair.0, a);
    bump(&mut pair.1, b);
}
