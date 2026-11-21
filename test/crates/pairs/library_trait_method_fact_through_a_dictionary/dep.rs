//! A library's trait whose generic method's impl asks a fact of the
//! method's own type parameter, rand 0.7.3's `SliceRandom` for `[T]`, whose
//! `choose_weighted` asks its `X`'s size: the library's contract lists it,
//! and every call through a dictionary of the trait gives it (#2285).
pub trait SliceRandom {
    type Item;
    fn choose(&self, at: usize) -> Option<&Self::Item>;
    fn sized<X: Clone>(&self, x: X) -> usize;
}

impl<T> SliceRandom for [T] {
    type Item = T;
    fn choose(&self, at: usize) -> Option<&T> {
        self.get(at)
    }
    fn sized<X: Clone>(&self, x: X) -> usize {
        let _kept = x.clone();
        std::mem::size_of::<X>() * 10 + self.len()
    }
}

/// The library's own generic code, given a consumer's dictionary.
pub fn sized_of<S: SliceRandom + ?Sized>(s: &S) -> usize {
    s.sized(1u16) + s.sized((1u8, 2u64))
}
