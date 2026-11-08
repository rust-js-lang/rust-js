// `append` is std's `merge` (#926): where a key's in both, the value the
// map held dropped, then the key given, as each turn of its loop ends; the
// first of the other's key at its end, its own binding's. As a held
// value's drop panics, the entry it held is gone: its key dropped, then
// the key given, whose value is leaked, then the rest of the other's, then
// the first's key. Of keys of a quiet order, of numbers, and of an order
// that's seen, std's nodes'; in generic code, its caller's map.
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};
use std::panic::{AssertUnwindSafe, catch_unwind};

/// A key of a quiet order, whose drop is shown.
struct Quiet(u32, &'static str);

impl Drop for Quiet {
    fn drop(&mut self) {
        println!("drop key {}{}", self.1, self.0);
    }
}

impl PartialEq for Quiet {
    fn eq(&self, other: &Quiet) -> bool {
        self.0 == other.0
    }
}

impl Eq for Quiet {}

impl PartialOrd for Quiet {
    fn partial_cmp(&self, other: &Quiet) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Quiet {
    fn cmp(&self, other: &Quiet) -> Ordering {
        self.0.cmp(&other.0)
    }
}

/// A key whose comparisons are shown.
struct Shown(u32, &'static str);

impl Drop for Shown {
    fn drop(&mut self) {
        println!("drop key {}{}", self.1, self.0);
    }
}

impl PartialEq for Shown {
    fn eq(&self, other: &Shown) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl Eq for Shown {}

impl PartialOrd for Shown {
    fn partial_cmp(&self, other: &Shown) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Shown {
    fn cmp(&self, other: &Shown) -> Ordering {
        println!("cmp {}{} {}{}", self.1, self.0, other.1, other.0);
        self.0.cmp(&other.0)
    }
}

/// A value whose drop is shown, and panics where it's `bad`.
struct Val(u32, bool);

impl Drop for Val {
    fn drop(&mut self) {
        println!("drop value {}", self.0);
        if self.1 {
            panic!("value {}", self.0);
        }
    }
}

fn appended<K: Ord>(key: impl Fn(u32, &'static str) -> K, number: impl Fn(&K) -> u32) {
    for bad in [0, 1, 3, 5] {
        println!("-- {bad}");
        let mut m = BTreeMap::new();
        for i in 0..6 {
            m.insert(key(i, "a"), Val(i, i == bad));
        }
        let mut other = BTreeMap::new();
        for i in [1, 3, 5, 7] {
            other.insert(key(i, "b"), Val(100 + i, false));
        }
        let appended = catch_unwind(AssertUnwindSafe(|| m.append(&mut other)));
        println!("appended {} len {} other {}", appended.is_ok(), m.len(), other.len());
        let held: Vec<String> = m.iter().map(|(k, v)| format!("{}:{}", number(k), v.0)).collect();
        println!("{}", held.join(" "));
        // What's left isn't dropped again: a `bad` value may be there.
        std::mem::forget(m);
    }
}

fn main() {
    appended(Quiet, |k| k.0);
    appended(|i, _| i, |k| *k);
    appended(Shown, |k| k.0);
    ended();
}

/// A quiet order's ends, `retain` and a set's `replace`, which a sorted
/// array answers where it holds them, asking nothing it can be seen asking.
fn ended() {
    let mut m = BTreeMap::new();
    for i in [5, 1, 3, 2, 4] {
        m.insert(Quiet(i, "q"), i);
    }
    println!("last {:?}", m.last_key_value().map(|(k, v)| (k.0, *v)));
    m.retain(|k, _| k.0 != 3);
    println!("first {:?}", m.first_key_value().map(|(k, v)| (k.0, *v)));
    println!("left {:?}", m.keys().map(|k| k.0).collect::<Vec<_>>());
    let mut s = BTreeSet::new();
    s.insert(Quiet(1, "a"));
    let old = s.replace(Quiet(1, "b")).map(|k| k.1);
    println!("replaced {old:?}, held {:?}", s.iter().map(|k| k.1).collect::<Vec<_>>());
}
