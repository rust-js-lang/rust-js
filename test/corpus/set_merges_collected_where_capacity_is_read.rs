// In a crate that reads a `Vec`'s capacity, a `Vec` collected of a B-tree
// set's `difference` or `union`, or of a hash set's `union`, grows by the
// merge's hint as std steps it: a B-tree difference's by which of its ways
// std takes, stitched, searched, or its own items alone; a union's the more
// of its sides; a hash union's, the larger set first, then what's left of
// the smaller. Each was refused (#1219). A hash union's items are sorted
// before they're shown, their order being unspecified.
use std::collections::{BTreeSet, HashSet};

fn show(label: &str, v: Vec<u32>) {
    println!("{label} {} {} {:?}", v.len(), v.capacity(), v.first());
}

fn sorted(mut v: Vec<u32>) -> Vec<u32> {
    v.sort();
    v
}

fn main() {
    let a: BTreeSet<u32> = (0..20).collect();
    let b: BTreeSet<u32> = (5..9).collect();
    let c: BTreeSet<u32> = (0..200).step_by(3).collect();
    let none: BTreeSet<u32> = BTreeSet::new();
    let ends: BTreeSet<u32> = (19..40).collect();
    show("stitched", a.difference(&b).copied().collect());
    show("searched", b.difference(&c).copied().collect());
    show("alone", c.difference(&none).copied().collect());
    show("an end", a.difference(&ends).copied().collect());
    show("union", a.union(&b).copied().collect());
    show("union of more", b.union(&c).copied().collect());
    show("union mapped", c.union(&a).map(|x| x * 2).collect());

    let h: HashSet<u32> = (0..10).collect();
    let g: HashSet<u32> = (5..30).collect();
    let hu: Vec<u32> = h.union(&g).copied().collect();
    println!("hash union {} {}", hu.len(), hu.capacity());
    println!("{:?}", sorted(hu));
    let gu: Vec<u32> = g.union(&h).copied().collect();
    println!("the other way {} {}", gu.len(), gu.capacity());
    let small: HashSet<u32> = (40..43).collect();
    let both = &small | &h;
    println!("{:?}", sorted(both.into_iter().collect()));
    let mut walked = h.union(&small);
    let first = walked.next().is_some();
    let rest: Vec<u32> = walked.copied().collect();
    println!("{first} {} {}", rest.len(), rest.capacity());
}
