//@ library-differs
// A hash map's `get_disjoint_mut`: each key's value, written in place
// through its `&mut`, a number's and a struct's alike, `None` of a key it
// doesn't hold, which may repeat; two keys that find one entry panic with
// std's message, the map as it was (#1276).
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::panic::{catch_unwind, AssertUnwindSafe};

#[derive(Debug)]
struct Word(String);
impl PartialEq for Word {
    fn eq(&self, other: &Word) -> bool {
        self.0.eq_ignore_ascii_case(&other.0)
    }
}
impl Eq for Word {}
impl Hash for Word {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.to_ascii_lowercase().hash(state);
    }
}

#[derive(Debug)]
struct Score {
    points: u32,
}

fn sorted(m: &HashMap<&str, u32>) -> Vec<(String, u32)> {
    let mut v: Vec<(String, u32)> = m.iter().map(|(k, v)| (k.to_string(), *v)).collect();
    v.sort();
    v
}

fn main() {
    let mut counts: HashMap<&str, u32> = HashMap::from([("a", 1), ("b", 2), ("c", 3)]);
    if let [Some(a), Some(c)] = counts.get_disjoint_mut(["a", "c"]) {
        std::mem::swap(a, c);
        *a += 10;
    }
    println!("{:?}", sorted(&counts));
    let [x, y, z] = counts.get_disjoint_mut(["z", "b", "z"]);
    println!("{x:?} {y:?} {z:?}");
    let r = catch_unwind(AssertUnwindSafe(|| {
        let [p, q] = counts.get_disjoint_mut(["a", "a"]);
        println!("{p:?} {q:?}");
    }));
    println!("{}", r.is_err());
    println!("{:?}", sorted(&counts));

    let mut scores: HashMap<u32, Score> = (1..4).map(|i| (i, Score { points: i * 10 })).collect();
    let [one, three] = scores.get_disjoint_mut([&1, &3]);
    if let (Some(one), Some(three)) = (one, three) {
        one.points += three.points;
        three.points = 0;
    }
    let mut seen: Vec<(u32, u32)> = scores.iter().map(|(k, s)| (*k, s.points)).collect();
    seen.sort();
    println!("{seen:?}");

    let mut words: HashMap<Word, u32> = HashMap::from([(Word("Apple".into()), 1), (Word("pear".into()), 2)]);
    let r = catch_unwind(AssertUnwindSafe(|| {
        words.get_disjoint_mut([&Word("APPLE".into()), &Word("apple".into())]);
    }));
    println!("{}", r.is_err());
    if let [Some(p), None] = words.get_disjoint_mut([&Word("PEAR".into()), &Word("plum".into())]) {
        *p += 5;
    }
    let mut w: Vec<(String, u32)> = words.iter().map(|(k, v)| (k.0.clone(), *v)).collect();
    w.sort();
    println!("{w:?}");
}
