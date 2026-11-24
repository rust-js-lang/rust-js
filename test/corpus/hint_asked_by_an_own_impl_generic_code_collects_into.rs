//@ library-differs
// Generic code's `collect()` into a `B: FromIterator`, `sum()` into an `S:
// Sum` and `product()` into a `P: Product`, or each called by name, of an
// iterator it makes itself, where its caller gives the crate's own impl,
// which asks `size_hint()` of what it's given: std's `Skip`, `Take` and
// `Filter` answer theirs, so the iterator is made knowing its hint where
// generic code makes it, as the impl it reaches is its callers' to choose
// (#2512).
use std::iter::{Product, Sum};

struct Hinted(usize, Option<usize>);

impl FromIterator<u32> for Hinted {
    fn from_iter<I: IntoIterator<Item = u32>>(items: I) -> Self {
        let (low, high) = items.into_iter().size_hint();
        Hinted(low, high)
    }
}

struct Room(usize, Option<usize>);

impl Sum<u32> for Room {
    fn sum<I: Iterator<Item = u32>>(items: I) -> Self {
        let (low, high) = items.size_hint();
        Room(low, high)
    }
}

impl Product<u32> for Room {
    fn product<I: Iterator<Item = u32>>(items: I) -> Self {
        let (low, high) = items.size_hint();
        Room(low + 100, high)
    }
}

fn gathered<B: FromIterator<u32>>(v: Vec<u32>) -> B {
    v.into_iter().take(2).collect()
}

fn gathered_by_name<B: FromIterator<u32>>(v: &[u32]) -> B {
    B::from_iter(v.iter().copied().filter(|x| x % 2 == 0))
}

fn total<S: Sum<u32>>(v: Vec<u32>) -> S {
    v.into_iter().skip(1).sum()
}

fn total_by_name<S: Sum<u32>>(v: &[u32]) -> S {
    S::sum(v.iter().copied().skip(2))
}

fn multiplied<P: Product<u32>>(v: Vec<u32>) -> P {
    v.into_iter().filter(|x| *x > 4).product()
}

// Public, so built as a library its callers are any crate's.
pub fn total_for_anyone<S: Sum<u32>>(v: Vec<u32>) -> S {
    v.into_iter().take(1).sum()
}

fn total_of<I: Iterator<Item = u32>, S: Sum<u32>>(items: I) -> S {
    items.sum()
}

// Asks it through the crate's own function it gives what it's given.
struct Measured(usize, Option<usize>);

fn measure<I: IntoIterator<Item = u32>>(items: I) -> (usize, Option<usize>) {
    items.into_iter().size_hint()
}

impl FromIterator<u32> for Measured {
    fn from_iter<I: IntoIterator<Item = u32>>(items: I) -> Self {
        let (low, high) = measure(items);
        Measured(low, high)
    }
}

fn firsts<B: FromIterator<u32>>(v: Vec<u32>) -> B {
    v.into_iter().take(1).collect()
}

struct Reserved(Vec<(usize, Option<usize>)>);

impl Extend<u32> for Reserved {
    fn extend<I: IntoIterator<Item = u32>>(&mut self, items: I) {
        self.0.push(items.into_iter().size_hint());
    }
}

fn filled<C: Extend<u32>>(into: &mut C, v: Vec<u32>) {
    into.extend(v.into_iter().skip(1));
}

fn main() {
    let Hinted(low, high) = gathered(vec![4, 5, 6]);
    println!("gathered {low} {high:?}");
    let Hinted(low, high) = gathered_by_name(&[4, 5, 6]);
    println!("gathered by name {low} {high:?}");
    let Room(low, high) = total(vec![4, 5, 6]);
    println!("total {low} {high:?}");
    let Room(low, high) = total_by_name(&[4, 5, 6]);
    println!("total by name {low} {high:?}");
    let Room(low, high) = multiplied(vec![4, 5, 6]);
    println!("multiplied {low} {high:?}");
    let Room(low, high) = total_for_anyone(vec![4, 5, 6]);
    println!("total for anyone {low} {high:?}");
    let Room(low, high) = total_of(vec![4, 5, 6].into_iter().skip(1));
    println!("total of {low} {high:?}");
    let mut reserved = Reserved(Vec::new());
    filled(&mut reserved, vec![4, 5, 6]);
    println!("filled {:?}", reserved.0);
    let Measured(low, high) = firsts(vec![7, 8, 9]);
    println!("measured {low} {high:?}");
    let plain: Vec<u32> = gathered(vec![4, 5, 6]);
    let summed: u32 = total(vec![4, 5, 6]);
    println!("{plain:?} {summed}");
}
