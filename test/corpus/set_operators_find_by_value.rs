// A set's operators, `&a | &b`, `&`, `-` and `^`, of items that are
// objects, a `BTreeSet`'s and a `HashSet`'s: each a set of its own class,
// its receiver's, which finds an item by its order or its hash, where it was
// a plain `Set` that found one by identity, so `contains` was `false` and
// `insert` added one it held again (#961).
use std::collections::{BTreeSet, HashSet};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct P(u32, u32);

fn main() {
    let a: BTreeSet<P> = [P(1, 1), P(2, 2), P(3, 3)].into_iter().collect();
    let b: BTreeSet<P> = [P(2, 2), P(4, 4)].into_iter().collect();
    let mut u = &a | &b;
    println!("{} {}", u.contains(&P(2, 2)), u.len());
    u.insert(P(1, 1));
    println!("{} {:?}", u.len(), u);
    let d = &a - &b;
    println!("{} {:?}", d.contains(&P(3, 3)), d);
    println!("{:?} {:?}", &a & &b, &a ^ &b);

    let h: HashSet<P> = a.iter().cloned().collect();
    let k: HashSet<P> = b.iter().cloned().collect();
    let mut v = &h | &k;
    println!("{} {}", v.contains(&P(2, 2)), v.len());
    v.insert(P(1, 1));
    println!("{}", v.len());
    let e = &h - &k;
    println!("{} {}", e.contains(&P(3, 3)), e.len());
    let t: HashSet<(u32, char)> = [(1, 'a'), (2, 'b')].into();
    let s: HashSet<(u32, char)> = [(2, 'b')].into();
    let x = &t & &s;
    println!("{} {}", x.contains(&(2, 'b')), x.len());
}
