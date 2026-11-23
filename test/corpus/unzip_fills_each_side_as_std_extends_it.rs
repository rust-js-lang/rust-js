// `unzip()` into any `Default + Extend` sides is std's own body, the pair's
// `Extend` by `default_extend`: a crate's own `Extend`, whose `extend` it
// calls an item at a time, `extend_one`'s default; a type parameter's,
// given a `HashSet` and a `String`; a set of what has a destructor, which
// drops a duplicate as it fills (#2395). And a crate's `extend` of a pair of
// collections, directly and through `C: Extend<(u32, char)>` (#2413).
use std::collections::{BTreeSet, HashSet};

#[derive(Default, Debug)]
struct Bag(Vec<u32>);

impl Extend<u32> for Bag {
    fn extend<I: IntoIterator<Item = u32>>(&mut self, items: I) {
        for item in items {
            println!("bag gets {item}");
            self.0.push(item);
        }
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Loud(u32);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn split<A: Default + Extend<u32>, B: Default + Extend<char>>(v: Vec<(u32, char)>) -> (A, B) {
    v.into_iter().unzip()
}

fn fill<C: Extend<(u32, char)>>(c: &mut C) {
    c.extend([(5, 'p'), (6, 'q')]);
}

fn main() {
    let pairs = vec![(1u32, 'a'), (2, 'b'), (1, 'c')];
    let (bag, letters): (Bag, Vec<char>) = pairs.clone().into_iter().unzip();
    println!("{bag:?} {letters:?}");
    let (bag, letters): (Bag, Vec<char>) = pairs.clone().into_iter().filter(|p| p.0 > 0).unzip();
    println!("{bag:?} {letters:?}");
    let (set, letters): (HashSet<u32>, String) = split(pairs.clone());
    let mut set: Vec<u32> = set.into_iter().collect();
    set.sort();
    println!("{set:?} {letters}");
    let (louds, n): (BTreeSet<Loud>, Vec<u32>) = vec![(Loud(1), 1u32), (Loud(1), 2), (Loud(2), 3)].into_iter().unzip();
    println!("{louds:?} {n:?}");
    let mut pair: (Vec<u32>, HashSet<char>) = Default::default();
    pair.extend([(3, 'x'), (4, 'y')]);
    let mut letters: Vec<char> = pair.1.into_iter().collect();
    letters.sort();
    println!("{:?} {letters:?}", pair.0);
    let mut pair: (Vec<u32>, String) = Default::default();
    fill(&mut pair);
    println!("{pair:?}");
}
