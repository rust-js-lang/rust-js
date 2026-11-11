//@ library-differs
// A trait default whose own where clause asks more of `Self::Item`,
// `Hash` for a `HashMap`'s keys, is given what that asks by its callers,
// never by the impl's dictionary: a blanket impl's dictionary of an
// iterator whose items have `Ord` but no `Hash` is made for `ordered`
// alone, though another default, `distinct`, calls `counts` under the same
// where clause (#1363).
use std::cmp::Ordering;
use std::collections::{BTreeMap, HashMap};
use std::hash::Hash;

trait Tally: Iterator + Sized {
    fn counts(self) -> HashMap<(usize, Self::Item), usize>
    where
        Self::Item: Hash + Eq,
    {
        let mut counts = HashMap::new();
        for (i, item) in self.enumerate() {
            *counts.entry((i % 2, item)).or_insert(0) += 1;
        }
        counts
    }

    fn distinct(self) -> usize
    where
        Self::Item: Hash + Eq,
    {
        self.counts().len()
    }

    fn ordered(self) -> BTreeMap<Self::Item, usize>
    where
        Self::Item: Ord,
    {
        let mut counts = BTreeMap::new();
        for item in self {
            *counts.entry(item).or_insert(0) += 1;
        }
        counts
    }
}

impl<I: Iterator> Tally for I {}

/// Ordered backwards, with no `Hash`.
#[derive(Debug, PartialEq, Eq)]
struct Backwards(u32);

impl PartialOrd for Backwards {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Backwards {
    fn cmp(&self, other: &Self) -> Ordering {
        other.0.cmp(&self.0)
    }
}

fn main() {
    let ordered = vec![Backwards(1), Backwards(3), Backwards(1)].into_iter().ordered();
    println!("{ordered:?}");

    let words = ["a", "b", "a", "a"];
    println!("{}", words.iter().distinct());
    let mut counts: Vec<_> = words.into_iter().counts().into_iter().collect();
    counts.sort();
    println!("{counts:?}");
    println!("{:?}", words.into_iter().ordered());
    println!("{}", (0..6).map(|n| n % 3).distinct());
}
