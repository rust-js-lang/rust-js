//! A trait extending `Iterator` whose defaults make maps keyed by
//! `Self::Item` and by a method's own type parameter: each map's class is
//! the method's caller's to give, which a consumer compiling the defaults
//! from their MIR asks as the library's copies do (#990).
use std::collections::HashMap;
use std::hash::Hash;

pub trait Tools: Iterator {
    fn counts(self) -> HashMap<Self::Item, usize>
    where
        Self: Sized,
        Self::Item: Hash + Eq,
    {
        let mut counts = HashMap::new();
        for item in self {
            *counts.entry(item).or_insert(0) += 1;
        }
        counts
    }

    fn keyed<K: Hash + Eq>(self, key: impl Fn(&Self::Item) -> K) -> HashMap<K, usize>
    where
        Self: Sized,
    {
        let mut keyed = HashMap::new();
        for item in self {
            *keyed.entry(key(&item)).or_insert(0) += 1;
        }
        keyed
    }
}

impl<I: Iterator> Tools for std::iter::Fuse<I> {}
