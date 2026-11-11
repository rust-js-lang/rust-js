//@ library-differs
// A map or a set keyed by a struct made of a type parameter, in generic
// code: one that derives what finds it, `Hash` and `PartialEq` of a hashed
// one, `Ord` of a B-tree, is found by its fields' classes in their order,
// as a tuple of them is, its caller giving each type parameter's; one whose
// own impls find it, by them, as a key of no type parameter is. A
// `PhantomData` field hashes nothing and is always equal. Each was refused
// (#1360).
use std::cell::Cell;
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;

#[derive(Hash, PartialEq, Eq, PartialOrd, Ord, Debug, Clone)]
struct Key<T>(u32, T);

#[derive(Hash, PartialEq, Eq, Debug)]
struct Named<T> {
    id: u32,
    tag: T,
}

#[derive(Hash, PartialEq, Eq, PartialOrd, Ord, Debug, Clone)]
struct Nested<T> {
    inner: Key<T>,
    rank: u8,
}

/// A typed id: its marker hashes nothing.
#[derive(Debug)]
struct Id<T> {
    raw: u32,
    of: PhantomData<T>,
}

impl<T> Id<T> {
    fn new(raw: u32) -> Self {
        Id { raw, of: PhantomData }
    }
}

impl<T> Clone for Id<T> {
    fn clone(&self) -> Self {
        Id::new(self.raw)
    }
}

impl<T> PartialEq for Id<T> {
    fn eq(&self, other: &Self) -> bool {
        self.raw == other.raw
    }
}

impl<T> Eq for Id<T> {}

impl<T> Hash for Id<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.raw.hash(state);
    }
}

/// A typed id whose finding is derived: its marker is no part of it.
#[derive(Hash, PartialEq, Eq, Debug)]
struct Tagged<T> {
    raw: u32,
    of: PhantomData<T>,
}

/// Equal and hashed by its id alone.
#[derive(Debug)]
struct Own<T> {
    id: u32,
    note: T,
}

impl<T> PartialEq for Own<T> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl<T> Eq for Own<T> {}

impl<T> Hash for Own<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

/// Ordered backwards by its rank alone.
#[derive(Debug, PartialEq, Eq)]
struct Ranked<T> {
    rank: u8,
    label: T,
}

impl<T: Eq> PartialOrd for Ranked<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<T: Eq> Ord for Ranked<T> {
    fn cmp(&self, other: &Self) -> Ordering {
        other.rank.cmp(&self.rank)
    }
}

/// Its `PartialEq` derived, its `Hash` its own, of `id` alone.
#[derive(PartialEq, Eq, Debug)]
struct Mixed<T> {
    id: u32,
    tag: T,
}

thread_local! {
    static MIXED_HASHED: Cell<u32> = const { Cell::new(0) };
}

impl<T> Hash for Mixed<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        MIXED_HASHED.set(MIXED_HASHED.get() + 1);
        self.id.hash(state);
    }
}

/// Hashed as nothing, equal by its id: two of one hash, told apart by `==`.
#[derive(Debug)]
struct Clash<T> {
    id: u32,
    note: T,
}

impl<T> PartialEq for Clash<T> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl<T> Eq for Clash<T> {}

impl<T> Hash for Clash<T> {
    fn hash<H: Hasher>(&self, _: &mut H) {}
}

/// Ordered by its derive, a B-tree's key read by its fields' names.
#[derive(PartialEq, Eq, PartialOrd, Ord, Debug)]
struct Graded<T> {
    grade: u8,
    who: T,
}

fn distinct<T: Hash + Eq>(keys: Vec<Key<T>>) -> usize {
    keys.into_iter().collect::<HashSet<_>>().len()
}

fn sorted<T: Ord + Clone>(keys: &[Key<T>]) -> Vec<Key<T>> {
    keys.iter().cloned().collect::<BTreeSet<_>>().into_iter().collect()
}

fn named<T: Hash + Eq>(keys: Vec<Named<T>>) -> usize {
    let mut counts = HashMap::new();
    for key in keys {
        *counts.entry(key).or_insert(0) += 1;
    }
    counts.len()
}

fn nested<T: Ord + Clone>(keys: &[Nested<T>]) -> Vec<Nested<T>> {
    let tree: BTreeMap<Nested<T>, usize> = keys.iter().map(|key| (key.clone(), 1)).collect();
    tree.into_keys().collect()
}

fn ids<T>(keys: &[Id<T>]) -> usize {
    keys.iter().cloned().collect::<HashSet<_>>().len()
}

fn own<T: Clone>(keys: Vec<Own<T>>) -> (usize, Vec<T>) {
    let notes = keys.iter().map(|key| key.note.clone()).collect();
    (keys.into_iter().collect::<HashSet<_>>().len(), notes)
}

fn ranked<T: Eq>(keys: Vec<Ranked<T>>) -> Vec<u8> {
    keys.into_iter()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .map(|key| key.rank)
        .collect()
}

fn mixed<T: Eq>(keys: Vec<Mixed<T>>) -> usize {
    keys.into_iter().collect::<HashSet<_>>().len()
}

fn tagged<T: Hash + Eq>(raws: &[u32]) -> usize {
    (raws.iter())
        .map(|&raw| Tagged::<T> { raw, of: PhantomData })
        .collect::<HashSet<_>>()
        .len()
}

fn graded<T: Ord>(keys: Vec<Graded<T>>) -> Vec<Graded<T>> {
    keys.into_iter().collect::<BTreeSet<_>>().into_iter().collect()
}

/// Through another generic function, its class its caller's in turn.
fn passed<T: Hash + Eq>(keys: Vec<Key<T>>) -> usize {
    distinct(keys)
}

fn main() {
    println!("{}", distinct(vec![Key(1, "a"), Key(1, "a"), Key(2, "a")]));
    println!("{}", distinct(vec![Key(1, 5u8), Key(1, 6)]));
    println!("{}", distinct(vec![Key(1, (2, 'x')), Key(1, (2, 'x'))]));
    println!("{:?}", sorted(&[Key(2, "b"), Key(1, "z"), Key(1, "a"), Key(2, "b")]));
    println!(
        "{}",
        named(vec![
            Named { id: 1, tag: 'x' },
            Named { id: 1, tag: 'x' },
            Named { id: 1, tag: 'y' }
        ])
    );
    let a = Nested {
        inner: Key(2, 'q'),
        rank: 1,
    };
    let b = Nested {
        inner: Key(1, 'z'),
        rank: 9,
    };
    println!("{:?}", nested(&[a.clone(), b, a]));
    println!("{}", ids::<String>(&[Id::new(4), Id::new(4), Id::new(5)]));
    println!(
        "{:?}",
        own(vec![
            Own { id: 1, note: "a" },
            Own { id: 1, note: "b" },
            Own { id: 2, note: "c" }
        ])
    );
    println!(
        "{:?}",
        ranked(vec![
            Ranked { rank: 1, label: "x" },
            Ranked { rank: 3, label: "y" },
            Ranked { rank: 1, label: "z" }
        ])
    );
    println!(
        "{}",
        mixed(vec![
            Mixed { id: 1, tag: 'a' },
            Mixed { id: 1, tag: 'a' },
            Mixed { id: 2, tag: 'b' }
        ])
    );
    println!("{}", tagged::<String>(&[3, 3, 4]));
    println!("{}", MIXED_HASHED.get() > 0);
    let clashes = vec![
        Named {
            id: 1,
            tag: Clash { id: 1, note: "a" },
        },
        Named {
            id: 1,
            tag: Clash { id: 2, note: "b" },
        },
        Named {
            id: 1,
            tag: Clash { id: 1, note: "c" },
        },
    ];
    let note = clashes[2].tag.note;
    println!("{note} {}", named(clashes));
    println!(
        "{:?}",
        graded(vec![
            Graded { grade: 2, who: 'b' },
            Graded { grade: 1, who: 'z' },
            Graded { grade: 2, who: 'a' }
        ])
    );
    println!("{}", passed(vec![Key(7, "s"), Key(7, "s")]));
}
