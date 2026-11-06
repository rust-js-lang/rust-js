// An `extend` of a `String` by `char`s or of a hash table asks its
// iterator's `size_hint` once, before it steps it, and reserves by its
// lower bound, as std's does (#849): `String`'s `Extend<char>`
// (string.rs), hashbrown's `extend`, half of it rounded up where the table
// has entries already, `HashSet`'s through its map's. One of a `String` by
// strings, or of a B-tree, asks none.
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

struct Loud {
    n: u32,
}

impl Iterator for Loud {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
        if self.n == 0 {
            return None;
        }
        self.n -= 1;
        Some(self.n)
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        print!("[hint {}] ", self.n);
        (self.n as usize, Some(self.n as usize))
    }
}

fn loud(n: u32) -> Loud {
    Loud { n }
}

fn main() {
    let mut s = String::from("ab");
    s.extend(loud(3).map(|x| char::from(b'c' + x as u8)));
    println!("chars {s}");
    let mut s = String::new();
    s.extend(loud(2).map(|x| x.to_string()));
    println!("strings {s}");
    let mut s = String::new();
    let cs = ['x', 'y'];
    s.extend(loud(2).map(|x| &cs[x as usize]));
    println!("char refs {s}");
    let mut m: HashMap<u32, u32> = HashMap::new();
    m.extend(loud(4).map(|x| (x, x)));
    m.extend(loud(3).map(|x| (x + 10, x)));
    println!("map {}", m.len());
    let mut set: HashSet<u32> = HashSet::new();
    set.extend(loud(3));
    println!("set {}", set.len());
    let mut b: BTreeMap<u32, u32> = BTreeMap::new();
    b.extend(loud(3).map(|x| (x, x)));
    let mut bs: BTreeSet<u32> = BTreeSet::new();
    bs.extend(loud(3));
    println!("btrees {} {}", b.len(), bs.len());
}
