//! A library trait whose default asks `size_of::<Self>()` and
//! `align_of::<Self>()`, its own impls' dictionaries giving them: a
//! constant of `Fixed`, the layout their makers give of `Wrap<T>`'s
//! (#2290).

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

pub struct Fixed(pub u32, pub u8);

impl Sz for Fixed {
    fn sz(&self) -> usize {
        7
    }
}

pub fn through<S: Sz>(s: &S) -> usize {
    s.al()
}
