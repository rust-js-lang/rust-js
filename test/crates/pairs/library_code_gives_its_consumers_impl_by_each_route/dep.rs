//! A library's generic code that makes an iterator and gives it to a `Sum`
//! its caller chooses, by each route a consumer reaches it: an inherent
//! method, a private function behind a public one, a closure, and a trait's
//! default with no type parameters of its own, called through the
//! dictionary the library exports as its own code uses it (#2518).
use std::iter::Sum;

pub struct Bin(pub Vec<u32>);

impl Bin {
    pub fn total<S: Sum<u32>>(&self) -> S {
        self.0.iter().copied().skip(1).sum()
    }
}

fn private_sum<S: Sum<u32>>(v: &[u32]) -> S {
    v.iter().copied().take(2).sum()
}

pub fn through_private<S: Sum<u32>>(v: &[u32]) -> S {
    private_sum(v)
}

pub fn in_closure<S: Sum<u32>>(v: &[u32]) -> Vec<S> {
    [1, 2].iter().map(|&n| v.iter().copied().skip(n).sum()).collect()
}

pub trait Tools: Iterator {
    fn tally(&self) -> Self::Item
    where
        Self::Item: Sum<u32>,
    {
        vec![1u32, 2, 3].into_iter().skip(1).sum()
    }
}

impl<T: Iterator> Tools for T {}

pub fn own_tally(v: Vec<u32>) -> u32 {
    v.into_iter().tally()
}
