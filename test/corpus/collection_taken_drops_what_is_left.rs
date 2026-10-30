// What a `take` or a `skip` of an owning iterator leaves of a collection's
// items is dropped, as Rust drops them: a `skip`'s as it passes them, a
// `take`'s with the iterator, at its statement's end, after what's taken
// is mapped. Of a `HashMap`'s `into_values()`, `into_keys()` and
// `into_iter()`, a `HashSet`'s, a B-tree's, a `VecDeque`'s and a `Vec`'s
// (#432). A hashed map's order is its own, so each counts its drops.
use std::cell::Cell;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet, VecDeque};

thread_local! {
    static DROPS: Cell<u32> = const { Cell::new(0) };
}

#[derive(PartialEq, Eq, Hash, PartialOrd, Ord)]
struct Loud(u32);

impl Drop for Loud {
    fn drop(&mut self) {
        DROPS.with(|d| d.set(d.get() + 1));
    }
}

/// How many were dropped since it was last asked.
fn drops() -> u32 {
    DROPS.with(|d| d.replace(0))
}

/// How many were dropped so far, still counted.
fn dropped() -> u32 {
    DROPS.with(|d| d.get())
}

fn limit() -> usize {
    1
}

fn by_number() -> HashMap<u32, Loud> {
    (0..3).map(|i| (i, Loud(i))).collect()
}

fn by_loud() -> HashMap<Loud, u32> {
    (0..3).map(|i| (Loud(i), i)).collect()
}

fn mapped(l: Loud) -> u32 {
    let seen = dropped();
    drop(l);
    seen
}

fn main() {
    let values: Vec<u32> = by_number().into_values().take(limit()).map(mapped).collect();
    println!("values {values:?} {}", drops());
    let keys: Vec<u32> = by_loud().into_keys().take(limit()).map(mapped).collect();
    println!("keys {keys:?} {}", drops());
    let entries: Vec<u32> = by_number()
        .into_iter()
        .take(limit())
        .map(|(_, l)| mapped(l))
        .collect();
    println!("entries {entries:?} {}", drops());
    let set: HashSet<Loud> = (0..3).map(Loud).collect();
    let items: Vec<u32> = set.into_iter().take(limit()).map(mapped).collect();
    println!("set {items:?} {}", drops());

    let tree: BTreeMap<u32, Loud> = (0..3).map(|i| (i, Loud(i))).collect();
    let tree_values: Vec<u32> = tree.into_values().take(limit()).map(mapped).collect();
    println!("tree values {tree_values:?} {}", drops());
    let tree_set: BTreeSet<Loud> = (0..3).map(Loud).collect();
    let tree_items: Vec<u32> = tree_set.into_iter().take(limit()).map(mapped).collect();
    println!("tree set {tree_items:?} {}", drops());
    let deque: VecDeque<Loud> = (0..3).map(Loud).collect();
    let deque_items: Vec<u32> = deque.into_iter().take(limit()).map(mapped).collect();
    println!("deque {deque_items:?} {}", drops());
    let list: Vec<Loud> = (0..3).map(Loud).collect();
    let list_items: Vec<u32> = list.into_iter().take(limit()).map(mapped).collect();
    println!("vec {list_items:?} {}", drops());

    let skipped: Vec<u32> = by_number().into_values().skip(limit()).map(mapped).collect();
    println!("values skipped {skipped:?} {}", drops());
    let skipped_keys: Vec<u32> = by_loud().into_keys().skip(2).map(mapped).collect();
    println!("keys skipped {skipped_keys:?} {}", drops());
    let tree: BTreeMap<u32, Loud> = (0..3).map(|i| (i, Loud(i))).collect();
    let tree_skipped: Vec<u32> = tree.into_values().skip(limit()).map(mapped).collect();
    println!("tree values skipped {tree_skipped:?} {}", drops());
    let list: Vec<Loud> = (0..3).map(Loud).collect();
    let list_skipped: Vec<u32> = list.into_iter().skip(limit()).map(mapped).collect();
    println!("vec skipped {list_skipped:?} {}", drops());

    let counted = by_number().into_values().take(2).count();
    println!("values counted {counted} {}", drops());
}
