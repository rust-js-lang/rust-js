//@ library-differs
// A hash map's entry is one search, as std's is: `entry(k)` hashes its key
// once, and `or_insert`, `or_insert_with`, `or_default`, `and_modify` and
// what's done with the value they give hash nothing again; nor does
// `get_disjoint_mut`'s handle, `retain`, `values_mut`, `iter_mut` or
// `extract_if` (#1362); nor does a B-tree's `values_mut` compare its keys
// whose `Ord` is its own. Each map
// is made with room for what it holds, so std's table never grows and
// hashes its keys again, which is its own.
use std::cell::Cell;
use std::cmp::Ordering;
use std::collections::hash_map::Entry;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt::Debug;
use std::hash::{Hash, Hasher};

thread_local! {
    static HASHES: Cell<u32> = const { Cell::new(0) };
    static COMPARED: Cell<u32> = const { Cell::new(0) };
}

/// A word whose case is its own: equal and hashed by its letters alone.
#[derive(Clone, Copy, Debug)]
struct Word(&'static str);

impl Hash for Word {
    fn hash<H: Hasher>(&self, state: &mut H) {
        HASHES.with(|hashes| hashes.set(hashes.get() + 1));
        for byte in self.0.bytes() {
            state.write_u8(byte.to_ascii_lowercase());
        }
    }
}

impl PartialEq for Word {
    fn eq(&self, other: &Self) -> bool {
        self.0.eq_ignore_ascii_case(other.0)
    }
}

impl Eq for Word {}

/// A rank whose order counts its comparisons.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Rank(u32);

impl Ord for Rank {
    fn cmp(&self, other: &Self) -> Ordering {
        COMPARED.with(|compared| compared.set(compared.get() + 1));
        self.0.cmp(&other.0)
    }
}

impl PartialOrd for Rank {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

fn hashed(what: &str) {
    println!("{what}: {} hashed", HASHES.with(|hashes| hashes.replace(0)));
}

fn counted<T: Hash + Eq + Copy>(words: &[T]) -> Vec<u32> {
    let mut counts = HashMap::with_capacity(16);
    for &word in words {
        *counts.entry(word).or_insert(0) += 1;
    }
    hashed("counted");
    let mut counts: Vec<_> = counts.into_values().collect();
    counts.sort();
    counts
}

fn entries<T: Hash + Eq + Copy + Debug>(a: T, b: T, c: T) {
    let mut lengths: HashMap<T, usize> = HashMap::with_capacity(16);
    *lengths.entry(a).or_insert(1) += 1;
    hashed("or_insert, then +=");
    lengths.entry(b).or_insert_with(|| 5);
    hashed("or_insert_with");
    *lengths.entry(a).or_default() += 2;
    hashed("or_default, then +=");
    lengths.entry(c).and_modify(|n| *n += 1).or_insert(9);
    hashed("and_modify, or_insert");
    lengths.entry(c).and_modify(|n| *n += 1).or_insert(9);
    hashed("and_modify of one there");
    let mut names: HashMap<T, String> = HashMap::with_capacity(16);
    names.entry(a).or_insert_with_key(|k| format!("{k:?}")).push('!');
    hashed("or_insert_with_key, then push");
    names.entry(a).or_default().push('?');
    hashed("or_default, then push");
    match names.entry(b) {
        Entry::Occupied(there) => there.into_mut().push('o'),
        Entry::Vacant(missing) => missing.insert(String::from("new")).push('v'),
    }
    hashed("a vacant entry's insert");
    if let Entry::Occupied(mut there) = names.entry(b) {
        there.get_mut().push('o');
        println!("{:?} {:?}", there.key(), there.get());
    }
    hashed("an occupied entry's get_mut");
    for name in names.get_disjoint_mut([&a, &c]).into_iter().flatten() {
        name.push('1');
    }
    hashed("get_disjoint_mut");
    lengths.retain(|_, n| *n > 2);
    hashed("retain");
    for n in lengths.values_mut() {
        *n *= 10;
    }
    hashed("values_mut");
    for (key, n) in lengths.iter_mut() {
        *n += format!("{key:?}").len();
    }
    hashed("iter_mut");
    let mut taken: Vec<_> = lengths.extract_if(|_, n| *n > 100).map(|(_, n)| n).collect();
    taken.sort();
    println!("{taken:?}");
    hashed("extract_if");
    let mut seen = HashSet::with_capacity(16);
    println!("{} {}", seen.insert(a), seen.insert(c));
    hashed("a set's insert");
    println!("{}", seen.extract_if(|_| true).count());
    hashed("a set's extract_if");
    let mut lengths: Vec<_> = lengths.into_values().collect();
    lengths.sort();
    let mut names: Vec<_> = names.into_values().collect();
    names.sort();
    println!("{lengths:?} {names:?} {}", seen.len());
}

fn main() {
    let words = [Word("to"), Word("be"), Word("OR"), Word("not"), Word("To"), Word("BE")];
    println!("{:?}", counted(&words));
    println!("{:?}", counted(&[3, 1, 3, 2]));
    entries(Word("a"), Word("b"), Word("C"));
    entries(Word("a"), Word("A"), Word("b"));

    // Where the map's keys are known here.
    let mut counts: HashMap<Word, u32> = HashMap::with_capacity(16);
    for word in words {
        *counts.entry(word).or_insert(0) += 1;
    }
    hashed("counted here");
    let mut lists: HashMap<Word, Vec<usize>> = HashMap::with_capacity(16);
    for (i, word) in words.into_iter().enumerate() {
        lists.entry(word).or_default().push(i);
    }
    hashed("lists here");
    let mut counts: Vec<_> = counts.into_iter().map(|(w, n)| (w.0.to_lowercase(), n)).collect();
    counts.sort();
    let mut lists: Vec<_> = lists.into_values().collect();
    lists.sort();
    println!("{counts:?} {lists:?}");

    let mut ranked: BTreeMap<Rank, u32> = (0..5).map(|i| (Rank(i), i)).collect();
    COMPARED.with(|compared| compared.set(0));
    for score in ranked.values_mut() {
        *score += 1;
    }
    for (rank, score) in ranked.iter_mut() {
        *score *= rank.0 + 1;
    }
    let compared = COMPARED.with(|compared| compared.replace(0));
    println!("{ranked:?}: {compared} compared");
}
