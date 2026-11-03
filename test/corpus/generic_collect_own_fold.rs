//@ library-differs
// What generic code collects by `fold`, a `String`, a `HashSet` or a
// `HashMap`, asks the `fold` of the type it's given, the crate's own where
// its impl writes one, as std's `extend` steps it by `for_each` (#521): of
// a type parameter, an `impl Iterator` and an adapter of one; of a type
// with none, a `Vec`'s, its items as they come.
use std::collections::{HashMap, HashSet};

struct Letters(u8);

impl Iterator for Letters {
    type Item = char;

    fn next(&mut self) -> Option<char> {
        if self.0 >= 3 {
            return None;
        }
        self.0 += 1;
        Some((b'a' + self.0) as char)
    }

    fn fold<B, F: FnMut(B, char) -> B>(mut self, init: B, mut f: F) -> B {
        println!("own fold");
        let mut acc = init;
        while let Some(c) = self.next() {
            acc = f(acc, c);
        }
        acc
    }
}

struct Plain(u8);

impl Iterator for Plain {
    type Item = char;

    fn next(&mut self) -> Option<char> {
        if self.0 >= 2 {
            return None;
        }
        self.0 += 1;
        println!("plain next {}", self.0);
        Some((b'x' + self.0) as char)
    }
}

fn text<I: Iterator<Item = char>>(it: I) -> String {
    it.collect()
}

fn set<I: Iterator<Item = char>>(it: I) -> usize {
    it.collect::<HashSet<_>>().len()
}

fn map<I: Iterator<Item = char>>(it: I) -> Vec<(char, usize)> {
    let mut pairs: Vec<_> = it
        .enumerate()
        .map(|(i, c)| (c, i))
        .collect::<HashMap<_, _>>()
        .into_iter()
        .collect();
    pairs.sort();
    pairs
}

fn text_impl(it: impl Iterator<Item = char>) -> String {
    it.collect()
}

fn upper<I: Iterator<Item = char>>(it: I) -> String {
    it.map(|c| c.to_ascii_uppercase()).collect()
}

fn listed<I: Iterator<Item = char>>(it: I) -> Vec<char> {
    it.collect()
}

fn main() {
    println!("{}", text(Letters(0)));
    println!("{}", set(Letters(0)));
    println!("{:?}", map(Letters(0)));
    println!("{}", text_impl(Letters(0)));
    println!("{}", upper(Letters(0)));
    println!("{:?}", listed(Letters(0)));
    println!("{}", text(Letters(0).skip(1)));
    println!("{}", text(Plain(0)));
    println!("{}", upper(Plain(0)));
    println!("{}", text(vec!['p', 'q'].into_iter()));
    println!("{}", set("hello".chars()));
}
