// In a crate that reads a `Vec`'s capacity, a `Vec` grown by what isn't
// `TrustedLen`, std grows it by the iterator's exact length: a `map` of a
// `take(n)` of a map's values, which MIR holds in a temporary, and a `map`
// of a held `skip(1)` stepped from its back, each counted where it's made,
// `$taking` or `$skipping`. Each was refused beside a case that reads a
// capacity, "can't count" (#1317).
use std::collections::HashMap;

#[derive(Debug)]
struct Loud(u32);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn limit() -> usize {
    println!("limit");
    2
}

fn mapped(l: Loud) -> u32 {
    l.0 * 10
}

fn main() {
    let m: HashMap<u32, Loud> = (1..2).map(|n| (n, Loud(n))).collect();
    let v: Vec<u32> = m.into_values().take(limit()).map(mapped).collect();
    println!("{v:?} {}", v.capacity() >= v.len());
    // Six of eight, counted where they're made: std's room for them all
    // after the first, 6, where JS's own `take` and `map`, which know no
    // hint, would grow from 4 to 8.
    let h: HashMap<u32, u32> = (1..9).map(|n| (n, n)).collect();
    let x: Vec<u32> = h.into_values().take(limit() + 4).map(|n| n * 2).collect();
    println!("{} {}", x.len(), x.capacity());
    let n: HashMap<u32, String> = (1..40).map(|n| (n, n.to_string())).collect();
    let w: Vec<String> = n.into_values().take(10).map(|s| s + "!").collect();
    println!("{} {}", w.len(), w.capacity());
    let items: Vec<u32> = (1..41).collect();
    let s = items.into_iter().skip(1);
    let r: Vec<u32> = s.map(|x| x + 1).rev().collect();
    println!("{} {:?} {}", r.len(), &r[..3], r.capacity());
    let t = vec![Loud(7), Loud(8), Loud(9)].into_iter().skip(1);
    let u: Vec<u32> = t.map(mapped).rev().collect();
    println!("{u:?} {}", u.capacity() >= 2);
}
