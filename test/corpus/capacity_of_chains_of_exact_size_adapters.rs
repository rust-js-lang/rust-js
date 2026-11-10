// A `Vec` grown by a chain or a zip whose parts are exact-sized adapters of
// what JS steps as an array, a set's, a map's keys' or values', in a crate
// that reads a capacity: std grows it by the chain's hint, its parts'
// summed, a zip's the fewer, each part's its length. Each part is made
// knowing its hint where it's made, `$skipping`, `$taking`, `$enumerate`,
// and what it holds down to its source, `$iter`; a map of the chain keeps
// it, `$mapEnds` (#1216).

use std::collections::{BTreeMap, BTreeSet, HashSet};

#[derive(Debug)]
struct L(u32);
impl Clone for L {
    fn clone(&self) -> L {
        println!("clone {}", self.0);
        L(self.0 + 100)
    }
}
impl Drop for L {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}
// A `repeat`'s taken, of what drops, in a chain: each clone, and the one
// it holds dropped after.
fn repeated() {
    let a = [1u32, 2, 3];
    let v: Vec<u32> = std::iter::repeat(7u32).take(2).chain(a.iter().copied().skip(1)).collect();
    println!("{} {}", v.len(), v.capacity());
    let w: Vec<L> = std::iter::repeat(L(7)).take(2).chain(std::iter::once(L(1))).collect();
    println!("{} {}", w.len(), w.capacity());
    let mut x: Vec<L> = Vec::new();
    x.extend(std::iter::repeat(L(8)).take(2).chain(std::iter::once(L(2))));
    println!("{} {}", x.len(), x.capacity());
}

fn main() {
    let a: Vec<u32> = (0..10).collect();
    let rest = vec![7u32, 8, 9];
    let b: BTreeSet<u32> = (0..40).collect();
    let h: HashSet<u32> = (0..20).collect();
    let d: Vec<u32> = b.iter().skip(1).chain(b.iter().take(3)).copied().collect();
    println!("{} {}", d.len(), d.capacity());
    let e: Vec<u32> = b.iter().skip(5).copied().chain(rest.iter().copied()).collect();
    println!("{} {}", e.len(), e.capacity());
    let f: Vec<u32> = h.iter().skip(2).chain(a.iter().skip(1)).copied().collect();
    println!("{} {}", f.len(), f.capacity());
    let g: Vec<(u32, u32)> = b.iter().skip(1).copied().zip(a.iter().copied()).collect();
    println!("{} {}", g.len(), g.capacity());
    let k: Vec<u32> = b.iter().skip(30).chain(b.iter().skip(38)).chain(a.iter().take(2)).copied().collect();
    println!("{} {}", k.len(), k.capacity());
    let empty: BTreeSet<u32> = BTreeSet::new();
    let m: Vec<u32> = empty.iter().skip(1).chain(empty.iter()).copied().collect();
    println!("{} {}", m.len(), m.capacity());
    let mut v = vec![1u32, 2];
    v.extend(b.iter().skip(35).chain(b.iter().take(2)).copied());
    println!("{} {}", v.len(), v.capacity());
    let map: BTreeMap<u32, u32> = (0..30).map(|x| (x, x * 2)).collect();
    let n: Vec<u32> = map.keys().skip(3).chain(map.values().skip(27)).copied().collect();
    println!("{} {}", n.len(), n.capacity());
    let p: Vec<(u32, u32)> = b.iter().skip(10).copied().zip(b.iter().skip(20).copied()).collect();
    println!("{} {}", p.len(), p.capacity());
    let q: Vec<(usize, u32)> = b.iter().copied().enumerate().skip(36).chain(a.iter().copied().enumerate().take(2)).collect();
    println!("{} {}", q.len(), q.capacity());
    let r: Vec<u32> = b.iter().take(4).chain(h.iter().take(3)).map(|x| x + 1).collect();
    println!("{} {}", r.len(), r.capacity());
    let s: Vec<&u32> = a.iter().skip(1).chain(a.iter().filter(|x| **x > 5)).collect();
    println!("{} {}", s.len(), s.capacity());
    repeated();
}
