// A `FlatMap` held is std's (ADR 0903): of a function giving `Vec`s,
// arrays and `Option`s, stepped from either end, folded both ways, sized as
// std sizes it, its function run as each item is reached and dropped with
// it; held in a struct and given back from generic code. One made in place
// stays the JS iterator it is.
use std::iter::FlatMap;

struct Loud(&'static str);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

struct Pairs<I: Iterator<Item = u32>> {
    pairs: FlatMap<I, [u32; 2], fn(u32) -> [u32; 2]>,
}

fn pair(n: u32) -> [u32; 2] {
    [n, n * 10]
}

fn pairs<I: Iterator<Item = u32>>(it: I) -> FlatMap<I, [u32; 2], fn(u32) -> [u32; 2]> {
    it.flat_map(pair as fn(u32) -> [u32; 2])
}

fn main() {
    // Of `Vec`s, from both ends, what's left sized.
    let mut runs = (1..4).flat_map(|n| (0..n).collect::<Vec<_>>());
    println!("{:?} {:?}", runs.next(), runs.size_hint());
    println!("{:?} {:?}", runs.next_back(), runs.size_hint());
    println!("{:?}", runs.collect::<Vec<_>>());

    // Of arrays, whose constant size bounds what's left.
    let mut doubled = [1, 2, 3].into_iter().flat_map(|n| [n, n]);
    println!("{:?}", doubled.size_hint());
    doubled.next_back();
    println!("{:?} {}", doubled.size_hint(), doubled.rfold(0, |a, x| a * 10 + x));

    // Of `Option`s, one item at most each.
    let mut evens = (1..7).flat_map(|n| (n % 2 == 0).then_some(n));
    println!("{:?} {:?}", evens.next(), evens.size_hint());
    println!("{}", evens.fold(0, |a, x| a + x));

    // Its function holds what it drops with it, after its items.
    let tag = Loud("tag");
    let mut tagged = vec![1, 2].into_iter().flat_map(move |n| {
        println!("map {n} {}", tag.0);
        vec![n; 2]
    });
    println!("took {:?}", tagged.next());
    drop(tagged);
    println!("dropped");

    // Held in a struct, made by generic code.
    let mut held = Pairs { pairs: pairs(vec![4, 5].into_iter()) };
    println!("{:?} {:?}", held.pairs.next(), held.pairs.size_hint());
    println!("{:?}", held.pairs.collect::<Vec<_>>());

    // Made in place, stepped where it's made.
    let total: u32 = (1..4).flat_map(|n| vec![n; n as usize]).sum();
    println!("{total}");
}
