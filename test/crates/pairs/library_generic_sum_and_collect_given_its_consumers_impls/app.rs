//! The consumer's own `Sum` and `FromIterator`, each asking the hint of what
//! the library's generic code gives it: std's `Take` and `Skip` know theirs,
//! `(1, Some(1))` and `(2, Some(2))`.
use std::iter::Sum;

struct Room(usize, Option<usize>);

impl Sum<u32> for Room {
    fn sum<I: Iterator<Item = u32>>(items: I) -> Self {
        let (low, high) = items.size_hint();
        Room(low, high)
    }
}

struct Hinted(usize, Option<usize>);

impl FromIterator<u32> for Hinted {
    fn from_iter<I: IntoIterator<Item = u32>>(items: I) -> Self {
        let (low, high) = items.into_iter().size_hint();
        Hinted(low, high)
    }
}

// Its own generic code hands its `S` on, which its caller makes a `Room`.
fn first_two_of<S: Sum<u32>>(v: &[u32]) -> S {
    dep::first_two(v)
}

pub fn main() {
    let Room(low, high) = dep::total_for_anyone(vec![4, 5, 6]);
    println!("total {low} {high:?}");
    let Hinted(low, high) = dep::gathered_for_anyone(&[4, 5, 6]);
    println!("gathered {low} {high:?}");
    let Room(low, high) = first_two_of(&[4, 5, 6]);
    println!("first two {low} {high:?}");
}
