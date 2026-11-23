// `partition` into a side its helper doesn't fill as std does, the crate's
// own `Extend`, a type parameter's, a set of what has a destructor, is
// std's own body compiled into `@rust-js/std`: each side's `Default`, the
// left's first, then each item's `extend_one`, std's default its `extend`
// of a `Some` of it, as native Rust calls them; a set's duplicate dropped as
// it's filled (#2387, #2378).
use std::collections::{BTreeSet, HashSet};

#[derive(Default, Debug)]
struct Bag(Vec<u32>, usize);

impl Extend<u32> for Bag {
    fn extend<I: IntoIterator<Item = u32>>(&mut self, items: I) {
        self.1 += 1;
        for item in items {
            println!("extend {item}");
            self.0.push(item * 10);
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

fn split<B: Default + Extend<u32>>(v: Vec<u32>) -> (B, B) {
    v.into_iter().partition(|&n| n > 1)
}

fn main() {
    let (big, small): (Bag, Bag) = vec![1u32, 2, 3].into_iter().partition(|&n| n > 1);
    println!("{big:?} {small:?}");
    let (a, b): (Bag, Bag) = split(vec![3, 1]);
    println!("{a:?} {b:?}");
    let (c, d): (HashSet<u32>, HashSet<u32>) = split(vec![1, 2, 2]);
    println!("{} {}", c.len(), d.len());
    let (e, f): (BTreeSet<u32>, BTreeSet<u32>) = split(vec![3, 2, 2, 1]);
    println!("{e:?} {f:?}");
    let (g, h): (Vec<u32>, Vec<u32>) = split(vec![2, 1, 2]);
    println!("{g:?} {h:?}");
    let loud = vec![Loud(2), Loud(1), Loud(2), Loud(3)];
    let (high, low): (BTreeSet<Loud>, BTreeSet<Loud>) = loud.into_iter().partition(|l| {
        println!("keep {}", l.0);
        l.0 > 1
    });
    println!("{high:?} {low:?}");
}
