// Generic code collecting into a type parameter, `B: FromIterator<A>`, as
// either's `Either` forwards `collect` (ADR 0446): a std collection's, text,
// a fallible one's and the crate's own.
use std::collections::{BTreeMap, HashSet};
use std::iter::FromIterator;

fn gather<B: FromIterator<u32>>(n: u32) -> B {
    (1..=n).map(|x| x * 2).collect()
}

fn made<B: FromIterator<char>>(text: &str) -> B {
    B::from_iter(text.chars().rev())
}

#[derive(Debug)]
struct Bag(Vec<u32>);

impl FromIterator<u32> for Bag {
    fn from_iter<I: IntoIterator<Item = u32>>(items: I) -> Self {
        Bag(items.into_iter().filter(|x| x % 3 != 0).collect())
    }
}

fn parsed<B: FromIterator<Result<u32, String>>>(words: &[&str]) -> B {
    words.iter().map(|w| w.parse::<u32>().map_err(|e| e.to_string())).collect()
}

fn main() {
    let v: Vec<u32> = gather(3);
    let s: HashSet<u32> = gather(2);
    let b: Bag = gather(4);
    let t: String = made("abc");
    let pairs: BTreeMap<u32, u32> = gather::<Vec<u32>>(2).into_iter().map(|x| (x, x + 1)).collect();
    let ok: Result<Vec<u32>, String> = parsed(&["1", "2"]);
    let bad: Result<Vec<u32>, String> = parsed(&["1", "x"]);
    println!("{v:?} {} {b:?} {t} {pairs:?} {ok:?} {bad:?}", s.len());
}
