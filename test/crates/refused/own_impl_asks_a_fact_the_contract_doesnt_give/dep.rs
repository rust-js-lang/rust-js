//! A library's trait with a generic method none of whose impls here asks a
//! fact of the method's own type parameter, and generic code of its own
//! that calls it through the trait's dictionary, so it gives none.
pub trait Pick {
    fn pick<X: Clone>(&self, x: X) -> usize;
}

impl Pick for u8 {
    fn pick<X: Clone>(&self, x: X) -> usize {
        let _kept = x.clone();
        usize::from(*self)
    }
}

pub fn pick_of<P: Pick>(p: &P) -> usize {
    p.pick(3u64)
}
