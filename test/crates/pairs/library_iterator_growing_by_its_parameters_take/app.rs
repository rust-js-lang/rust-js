//! A consumer of a library's iterator whose `next` asks whether the `I` it
//! holds is `TrustedLen` (#2327): the library's `Powerset` folded, of a
//! range, which is, and of the crate's own iterator, which isn't; and the
//! library's `Combinations` held by the crate's generic code, given where a
//! generic iterator goes and as an impl's `IntoIter`, whose callers each
//! say what their `I` is.
use dep::Itertools;
use dep::combinations::{Combinations, combinations};

struct Countdown(u32);

impl Iterator for Countdown {
    type Item = u32;

    fn next(&mut self) -> Option<u32> {
        if self.0 == 0 {
            return None;
        }
        self.0 -= 1;
        Some(self.0)
    }
}

fn total<J: Iterator<Item = Vec<u32>>>(sets: J) -> usize {
    sets.map(|set| set.len()).sum()
}

fn total_of<I: Iterator<Item = u32>>(combs: Combinations<I>) -> usize {
    total(combs)
}

struct Source<I>(I);

impl<I: Iterator<Item = u32>> IntoIterator for Source<I> {
    type Item = Vec<u32>;
    type IntoIter = Combinations<I>;

    fn into_iter(self) -> Combinations<I> {
        combinations(self.0, 2)
    }
}

fn count_all<T: IntoIterator<Item = Vec<u32>>>(sets: T) -> usize {
    sets.into_iter().count()
}

pub fn main() {
    println!("{}", (0..4).powerset().fold(0, |sum, set| sum + set.len()));
    println!("{}", Countdown(4).powerset().fold(0, |sum, set| sum + set.len()));
    println!("{:?}", (1..4).powerset().collect::<Vec<_>>());
    println!("{}", total_of(combinations(0..5, 2)));
    println!("{}", total_of(combinations(Countdown(5), 3)));
    println!("{}", count_all(Source(0..4)));
    println!("{}", count_all(Source(Countdown(4))));
}
