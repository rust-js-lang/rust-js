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
    let n: HashMap<u32, String> = (1..40).map(|n| (n, n.to_string())).collect();
    let w: Vec<String> = n.into_values().take(10).map(|s| s + "!").collect();
    println!("{} {}", w.len(), w.capacity());
    let s = vec![1u32, 2, 3, 4, 5].into_iter().skip(1);
    let r: Vec<u32> = s.map(|x| x + 1).rev().collect();
    println!("{r:?} {}", r.capacity() >= 4);
    let t = vec![Loud(7), Loud(8), Loud(9)].into_iter().skip(1);
    let u: Vec<u32> = t.map(mapped).rev().collect();
    println!("{u:?} {}", u.capacity() >= 2);
}
