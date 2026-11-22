//! A library's generic functions that give back what an `IntoIterator`
//! makes, itself and in an `Option`, its own impl whose `IntoIter` is a
//! range, and generic code over a reference's `IntoIterator`: each
//! `IntoIter` is its type's own value, a range a range, in the library's
//! generic code and its consumer's concrete code alike (#2331).
pub fn it<T: IntoIterator>(t: T) -> T::IntoIter {
    t.into_iter()
}

pub fn some<T: IntoIterator>(t: T) -> Option<T::IntoIter> {
    Some(t.into_iter())
}

pub struct Bag(pub Vec<u32>);

impl IntoIterator for Bag {
    type Item = u32;
    type IntoIter = std::ops::Range<u32>;
    fn into_iter(self) -> std::ops::Range<u32> {
        0..self.0.len() as u32
    }
}

pub fn total<C>(c: &C) -> u32
where
    for<'a> &'a C: IntoIterator<Item = u32>,
{
    let mut s = 0;
    for x in c {
        s += x;
    }
    s
}
