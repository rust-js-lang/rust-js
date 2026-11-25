// A library's generic impls of its trait, whose type parameters are each
// impl's own: rust-js asked whether JS names an impl by its visibility,
// which a library's impl has none of in its metadata, and rustc ICEd
// compiling the app (#2590).
use std::collections::HashMap;
use std::hash::{BuildHasher, Hash};

pub trait Made: Clone + 'static {
    fn made(n: u32) -> Self;

    fn smaller(&self) -> Box<dyn Iterator<Item = Self>> {
        Box::new(std::iter::empty())
    }
}

impl Made for u32 {
    fn made(n: u32) -> u32 {
        n
    }
}

impl<A: Made> Made for Vec<A> {
    fn made(n: u32) -> Vec<A> {
        (0..n).map(A::made).collect()
    }

    fn smaller(&self) -> Box<dyn Iterator<Item = Vec<A>>> {
        let mut shorter = self.clone();
        shorter.pop();
        Box::new(std::iter::once(shorter))
    }
}

impl<A: Made, B: Made> Made for (A, B) {
    fn made(n: u32) -> (A, B) {
        (A::made(n), B::made(n + 1))
    }
}

impl<K: Made + Eq + Hash, V: Made, S: BuildHasher + Default + Clone + 'static> Made for HashMap<K, V, S> {
    fn made(n: u32) -> Self {
        let pairs: Vec<(K, V)> = Made::made(n);
        pairs.into_iter().collect()
    }
}

pub fn made<A: Made>(n: u32) -> A {
    A::made(n)
}
