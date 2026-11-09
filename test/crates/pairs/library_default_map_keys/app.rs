//! A consumer's impls of the library's `Tools`, of a known item and of a
//! type parameter's, each map's class given by the caller, through a
//! generic caller too, and the library's own impl called here (#990).
use dep::Tools;
use std::hash::Hash;

struct Countdown(u8);

impl Iterator for Countdown {
    type Item = (u8, bool);

    fn next(&mut self) -> Option<(u8, bool)> {
        if self.0 == 0 {
            return None;
        }
        self.0 -= 1;
        Some((self.0 % 2, self.0 > 2))
    }
}

impl Tools for Countdown {}

struct Wrap<I>(I);

impl<I: Iterator> Iterator for Wrap<I> {
    type Item = I::Item;

    fn next(&mut self) -> Option<I::Item> {
        self.0.next()
    }
}

impl<I: Iterator> Tools for Wrap<I> {}

fn distinct<T: Tools>(items: T) -> usize
where
    T::Item: Hash + Eq,
{
    items.counts().len()
}

/// A map's entries in order, of each map's own types.
macro_rules! sorted {
    ($map:expr) => {{
        let mut entries: Vec<_> = $map.into_iter().collect();
        entries.sort();
        entries
    }};
}

pub fn main() {
    println!("{:?}", sorted!(Countdown(5).counts()));
    println!("{:?}", sorted!(Wrap(vec![3, 1, 3].into_iter()).counts()));
    println!("{:?}", sorted!(Wrap("abca".chars()).keyed(|c| c.is_ascii_lowercase())));
    println!("{} {}", distinct(Wrap("xyx".chars())), distinct(Countdown(4)));
    println!("{:?}", sorted!("hello".chars().fuse().counts()));
    println!("{:?}", sorted!((0..7u32).fuse().keyed(|n| n % 3)));
}
