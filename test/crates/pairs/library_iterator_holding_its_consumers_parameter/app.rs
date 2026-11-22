//! A consumer of a library's iterator that holds its `I` (#1420): the
//! library's `Powerset`, whose impl it compiles itself, of a range, of the
//! crate's own iterator and of `String`s, stepped by `next`, `nth` and
//! `fold`; and the library's `Combinations` held by the crate's own
//! generic code, stepped by its `next` and its `nth`, given where a generic
//! iterator goes, as an impl's `IntoIter`, and by a trait's default an impl
//! for it keeps.
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

struct Pairs<I: Iterator> {
    combs: Combinations<I>,
}

impl<I: Iterator> Iterator for Pairs<I>
where
    I::Item: Clone,
{
    type Item = Vec<I::Item>;

    fn next(&mut self) -> Option<Vec<I::Item>> {
        self.combs.next()
    }

    fn nth(&mut self, n: usize) -> Option<Vec<I::Item>> {
        self.combs.nth(n)
    }
}

fn pairs<I: Iterator>(it: I) -> Pairs<I>
where
    I::Item: Clone,
{
    Pairs {
        combs: combinations(it, 2),
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

trait FirstTwo: Iterator {
    fn first_two(&mut self) -> (Option<Self::Item>, Option<Self::Item>) {
        (self.next(), self.next())
    }
}

impl<I: Iterator> FirstTwo for Combinations<I> where I::Item: Clone {}

pub fn main() {
    println!("{:?}", (0..3).powerset().collect::<Vec<_>>());
    let mut sets = (1..5).powerset();
    println!("{:?} {:?} {:?}", sets.next(), sets.nth(3), sets.nth(6));
    println!("{}", Countdown(4).powerset().fold(0, |sum, set| sum + set.len()));
    let words = ["a", "b", "c"].map(String::from);
    println!("{:?}", words.into_iter().powerset().last());
    println!("{:?}", pairs(Countdown(4)).collect::<Vec<_>>());
    let mut taken = pairs(0..4);
    println!("{:?} {:?}", taken.nth(2), taken.count());
    println!("{}", total_of(combinations(Countdown(5), 2)));
    println!("{}", count_all(Source(Countdown(4))));
    println!("{:?}", combinations(Countdown(4), 3).first_two());
}
