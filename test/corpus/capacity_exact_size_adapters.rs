// A `Vec` collected of an `ExactSizeIterator` that isn't `TrustedLen`, a
// B-tree's or a hash table's iterator skipped, enumerated, mapped, taken,
// stepped or reversed, in a crate that reads a capacity: std grows it by its
// hint's lower bound, its length, which JS's own `drop(n)` of an array's
// iterator knew nothing of (#1200).
use std::collections::{BTreeMap, BTreeSet, HashSet};

fn caps(n: u32) {
    let b: BTreeSet<u32> = (0..n).collect();
    let v: Vec<&u32> = b.iter().skip(1).collect();
    let e: Vec<(usize, &u32)> = b.iter().enumerate().skip(1).collect();
    let h: HashSet<u32> = (0..n).collect();
    let w: Vec<&u32> = h.iter().skip(1).collect();
    let m: BTreeMap<u32, u32> = (0..n).map(|i| (i, i)).collect();
    let k: Vec<&u32> = m.keys().skip(1).collect();
    let all: Vec<&u32> = b.iter().collect();
    let en: Vec<(usize, &u32)> = b.iter().enumerate().collect();
    let fused: Vec<&u32> = b.iter().skip(1).fuse().collect();
    let stepped: Vec<&u32> = b.iter().skip(1).step_by(2).collect();
    let reversed: Vec<&u32> = b.iter().rev().skip(1).collect();
    let mapped: Vec<u32> = b.iter().skip(1).map(|x| x * 2).collect();
    let taken: Vec<&u32> = b.iter().skip(1).take(30).collect();
    let owned: Vec<u32> = b.clone().into_iter().skip(1).collect();
    println!(
        "{n}: {} {} {} {} {} {} {} {} {} {} {} {}",
        v.capacity(),
        e.capacity(),
        w.capacity(),
        k.capacity(),
        all.capacity(),
        en.capacity(),
        fused.capacity(),
        stepped.capacity(),
        reversed.capacity(),
        mapped.capacity(),
        taken.capacity(),
        owned.capacity()
    );
}

// A chain of what's stepped as an array's `$iter`, whose hint is std's,
// adapted after.
fn chains(n: u32) {
    let b: BTreeSet<u32> = (0..n).collect();
    let a: Vec<u32> = (0..n).collect();
    let none = a.iter().filter(|x| **x > 100);
    let skipped: Vec<&u32> = b.iter().chain(none.clone()).skip(1).collect();
    let enumerated: Vec<(usize, &u32)> = b.iter().chain(none.clone()).enumerate().skip(2).collect();
    let mapped: Vec<u32> = b.iter().chain(none.clone()).map(|x| x + 1).collect();
    let stepped: Vec<&u32> = b.iter().chain(none.clone()).step_by(3).collect();
    let taken: Vec<&u32> = b.iter().chain(none).take(35).collect();
    println!(
        "{n}: {} {} {} {} {}",
        skipped.capacity(),
        enumerated.capacity(),
        mapped.capacity(),
        stepped.capacity(),
        taken.capacity()
    );
}

fn main() {
    for n in [0, 1, 2, 3, 5, 9, 40] {
        caps(n);
        chains(n);
    }
}
