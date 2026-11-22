//! A library trait whose default asks `size_of::<Self>()` and
//! `align_of::<Self>()`, with an impl of its own and a generic function
//! that calls the default through a dictionary (#2291).

pub trait Sz: Sized {
    fn sz(&self) -> usize;
    fn al(&self) -> usize {
        std::mem::align_of::<Self>() * 100 + std::mem::size_of::<Self>()
    }
}

pub struct Wrap<T>(pub [T; 2]);

impl<T> Sz for Wrap<T> {
    fn sz(&self) -> usize {
        std::mem::size_of::<Self>()
    }
}

pub fn through<S: Sz>(s: &S) -> usize {
    s.al()
}
