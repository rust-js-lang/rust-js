//@ library-differs
// An enumeration of a type parameter's iterator that generic code makes,
// not one it's given as it is, mapped and collected into a `HashMap`: its
// source forward alone where its bounds give it no back, as rustc asks
// nothing more of it (ADR 0545, amended), its own `fold` the crate's.
use std::collections::HashMap;

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

fn positions<I: Iterator<Item = char>>(make: impl FnOnce() -> I) -> Vec<(char, usize)> {
    let it = make();
    let mut pairs: Vec<_> = it
        .enumerate()
        .map(|(i, c)| (c, i))
        .collect::<HashMap<_, _>>()
        .into_iter()
        .collect();
    pairs.sort();
    pairs
}

fn main() {
    println!("{:?}", positions(|| Letters(0)));
    println!("{:?}", positions(|| "xyz".chars()));
}
