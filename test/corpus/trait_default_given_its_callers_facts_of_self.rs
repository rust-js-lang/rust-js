// A trait's default copied into the crate's impl, `impl<T: Iterator>
// Tools for T`, given what its callers know of `Self` as a generic
// function's callers give it (#2011, #1956): whether it's a
// `FusedIterator`, so the default's `fuse()` passes each `next` on as
// std's does, a `FusedIterator` that gives items after a `None` too; and
// a `flatten()` of what's fused, whose inner `Fuse` is std's. Of a `Vec`'s
// items, a crate's iterator that isn't fused, and one with a destructor.
use std::iter::FusedIterator;

struct Blinks(u32);

impl Iterator for Blinks {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
        self.0 += 1;
        (self.0 % 2 == 0).then_some(self.0)
    }
}

impl FusedIterator for Blinks {}

struct Unfused(u32);

impl Iterator for Unfused {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
        self.0 += 1;
        (self.0 % 2 == 0).then_some(self.0)
    }
}

struct Loud(u32);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop Loud {}", self.0);
    }
}

impl Iterator for Loud {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
        self.0 += 1;
        (self.0 < 3).then_some(self.0)
    }
}

impl FusedIterator for Loud {}

pub trait Tools: Iterator {
    fn fused_four(self) -> Vec<Option<Self::Item>>
    where
        Self: Sized,
    {
        let mut it = self.fuse();
        (0..4).map(|_| it.next()).collect()
    }

    fn flattened(self) -> Vec<u32>
    where
        Self: Sized + Iterator<Item = Vec<u32>>,
    {
        self.flatten().collect()
    }
}

impl<T: Iterator> Tools for T {}

fn main() {
    println!("{:?}", Blinks(0).fused_four());
    println!("{:?}", Unfused(0).fused_four());
    println!("{:?}", vec![1, 2].into_iter().fused_four());
    println!("{:?}", Loud(0).fused_four());
    println!("{:?}", vec![vec![1, 2], vec![], vec![3]].into_iter().flattened());
}
