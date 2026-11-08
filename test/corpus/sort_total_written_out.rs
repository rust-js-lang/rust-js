// An `Ord` that writes `then` out by hand is a total order, so its sort
// stays JS's `sort` (#936): `if o != Equal { o } else { p }`, its `==` the
// other way round, and `match o { Equal => p, o => o }`, each through a
// `let`; and `if o != Equal { p } else { o }`, which isn't `then`, is
// std's own sort.
use std::cmp::Ordering;

#[derive(Debug, PartialEq, Eq)]
struct Word(String);

impl PartialOrd for Word {
    fn partial_cmp(&self, other: &Word) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Word {
    fn cmp(&self, other: &Word) -> Ordering {
        let by_length = self.0.len().cmp(&other.0.len());
        if by_length != Ordering::Equal {
            by_length
        } else {
            self.0.cmp(&other.0)
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
struct Pair(u32, u32);

impl PartialOrd for Pair {
    fn partial_cmp(&self, other: &Pair) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Pair {
    fn cmp(&self, other: &Pair) -> Ordering {
        let first = other.0.cmp(&self.0);
        match first {
            Ordering::Equal => self.1.cmp(&other.1),
            first => first,
        }
    }
}

fn main() {
    let mut words: Vec<Word> = "pear fig apple kiwi plum date lime"
        .split(' ')
        .map(|w| Word(w.to_string()))
        .collect();
    words.sort();
    println!("{words:?}");

    let mut pairs: Vec<Pair> = (0..30).map(|i| Pair(i % 4, (i * 7) % 11)).collect();
    pairs.sort();
    println!("{:?}", pairs.iter().map(|p| (p.0, p.1)).collect::<Vec<_>>());

    let mut by: Vec<(u32, u32)> = (0..25).map(|i| (i % 3, (i * 5) % 7)).collect();
    by.sort_by(|a, b| {
        let first = a.0.cmp(&b.0);
        if first == Ordering::Equal { b.1.cmp(&a.1) } else { first }
    });
    println!("{by:?}");

    // Not `then`: where the first differ, the second decides.
    let mut odd: Vec<(u32, u32)> = (0..20).map(|i| (i % 3, (i * 5) % 7)).collect();
    odd.sort_by(|a, b| {
        let first = a.0.cmp(&b.0);
        if first != Ordering::Equal { a.1.cmp(&b.1) } else { first }
    });
    println!("{odd:?}");
}
