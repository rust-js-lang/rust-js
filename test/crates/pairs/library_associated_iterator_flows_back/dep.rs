//! A library's trait whose associated type is an iterator, an impl of it
//! that gives a range, and generic code that steps one and gives one back,
//! each through the trait's dictionary: the value is its type's own, a
//! range a range, in the library's generic code and its consumer's concrete
//! code alike (#2330).
pub trait Source {
    type Out: DoubleEndedIterator<Item = u32>;
    fn make(&self, n: u32) -> Self::Out;
}

pub struct Ranged;

impl Source for Ranged {
    type Out = std::ops::Range<u32>;
    fn make(&self, n: u32) -> std::ops::Range<u32> {
        0..n
    }
}

pub fn given_back<S: Source>(s: &S, n: u32) -> S::Out {
    let mut it = s.make(n + 1);
    it.next();
    it
}

pub fn from_its_back<S: Source>(s: &S, n: u32) -> Vec<u32> {
    s.make(n).rev().collect()
}
