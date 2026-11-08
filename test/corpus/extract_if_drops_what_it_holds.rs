// A map's or a set's `extract_if` drops what it holds where it's dropped,
// its test then a B-tree's range, its bounds start then end, as std's
// `ExtractIf { pred, inner: { range, .. } }` drops them (#945): after a
// collect or a `count` that steps it to its end, and where a variable that
// holds it is dropped, once; owned bounds of a key with a destructor, of
// an order nothing sees and of one that can be seen.
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet, HashMap};

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
struct K(u32);

impl Drop for K {
    fn drop(&mut self) {
        println!("drop k{}", self.0);
    }
}

/// A key whose comparisons print, so its order can be seen.
#[derive(Debug)]
struct Seen(u32);

impl Drop for Seen {
    fn drop(&mut self) {
        println!("drop s{}", self.0);
    }
}

impl PartialEq for Seen {
    fn eq(&self, other: &Seen) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl Eq for Seen {}

impl PartialOrd for Seen {
    fn partial_cmp(&self, other: &Seen) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Seen {
    fn cmp(&self, other: &Seen) -> Ordering {
        println!("cmp {} {}", self.0, other.0);
        self.0.cmp(&other.0)
    }
}

/// What a test holds, dropped with it.
struct Held(&'static str);

impl Drop for Held {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn main() {
    let mut m: BTreeMap<K, u32> = (0..6).map(|i| (K(i), i)).collect();
    let taken: Vec<u32> = m.extract_if(K(1)..K(4), |_, v| *v % 2 == 0).map(|(_, v)| v).collect();
    println!("taken {taken:?}");
    let mut it = m.extract_if(K(10).., |_, _| true);
    println!("first {:?}", it.next().map(|(_, v)| v));
    drop(it);
    println!("after drop");
    let n = m.extract_if(..K(3), |k, _| k.0 == 1).count();
    println!("count {n}");
    let mut s: BTreeSet<K> = (0..4).map(K).collect();
    let all: Vec<u32> = s.extract_if(K(1)..K(3), |k| k.0 != 1).map(|k| k.0).collect();
    println!("set {all:?} {}", s.len());

    let mut seen: BTreeSet<Seen> = (0..14).map(Seen).collect();
    println!("made");
    let held = Held("test");
    let odd: Vec<u32> = seen
        .extract_if(Seen(3)..Seen(9), move |s| {
            let _ = &held;
            s.0 % 2 == 1
        })
        .map(|s| s.0)
        .collect();
    println!("odd {odd:?} {}", seen.len());

    let mut h: HashMap<u32, u32> = (0..4).map(|i| (i, i * 10)).collect();
    let held = Held("hashed");
    let mut big: Vec<u32> = h
        .extract_if(move |_, v| {
            let _ = &held;
            *v >= 20
        })
        .map(|(k, _)| k)
        .collect();
    big.sort();
    println!("hashed {big:?} {}", h.len());
    let held = Held("early");
    let it = h.extract_if(move |_, _| {
        let _ = &held;
        false
    });
    drop(it);
    println!("dropped early {}", h.len());
}
