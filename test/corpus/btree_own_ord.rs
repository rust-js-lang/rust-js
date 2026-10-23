// A B-tree of keys whose `Ord` is the crate's own finds a key by it, in
// its order, as std's does: an `$ordMap(cmp)` (ADR 0404). A generic one is
// given its class by its caller.
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Debug;

/// A task, the same as another of its id, the most urgent first.
#[derive(Debug, Clone)]
struct Task {
    id: u32,
    urgency: u8,
    name: &'static str,
}

impl PartialEq for Task {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for Task {}

impl PartialOrd for Task {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Task {
    fn cmp(&self, other: &Self) -> Ordering {
        other.id.cmp(&self.id)
    }
}

/// A score, the highest first: its `Ord` its own, its `Eq` derived.
#[derive(Debug, PartialEq, Eq)]
struct Score(u32);

impl PartialOrd for Score {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Score {
    fn cmp(&self, other: &Self) -> Ordering {
        other.0.cmp(&self.0)
    }
}

/// A size, the largest first.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Size {
    Small,
    Large,
}

impl PartialOrd for Size {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Size {
    fn cmp(&self, other: &Self) -> Ordering {
        (*other as u8).cmp(&(*self as u8))
    }
}

fn task(id: u32, urgency: u8, name: &'static str) -> Task {
    Task { id, urgency, name }
}

fn tally<K: Ord + Clone + Debug>(items: &[K]) -> BTreeMap<K, u32> {
    let mut counts = BTreeMap::new();
    for item in items {
        *counts.entry(item.clone()).or_insert(0) += 1;
    }
    counts
}

fn distinct<T: Ord>(items: Vec<T>) -> usize {
    items.into_iter().collect::<BTreeSet<T>>().len()
}

fn main() {
    let mut owners = BTreeMap::new();
    owners.insert(task(1, 3, "write"), "ana");
    owners.insert(task(4, 1, "test"), "bo");
    println!("{:?}", owners.insert(task(1, 9, "rewrite"), "cy"));
    println!("{:?}", owners.get(&task(4, 0, "")));
    println!("{:?}", owners.get_key_value(&task(1, 0, "")).map(|(t, o)| (t.name, *o)));
    println!("{:?}", owners.keys().map(|t| t.name).collect::<Vec<_>>());
    println!("{:?}", owners.first_key_value().map(|(t, _)| t.id));
    *owners.entry(task(2, 5, "ship")).or_insert("dee") = "eve";
    let copy = owners.clone();
    println!("{:?} {}", owners.remove(&task(2, 0, "")), copy.len());
    println!(
        "{:?}",
        copy.range(task(3, 0, "")..)
            .map(|(t, o)| (t.id, *o))
            .collect::<Vec<_>>()
    );

    let mut queue: BTreeSet<Task> = [task(7, 2, "a"), task(3, 8, "b")].into_iter().collect();
    println!("{} {}", queue.insert(task(7, 0, "c")), queue.len());
    println!("{:?}", queue.get(&task(7, 0, "")).map(|t| t.name));
    println!("{:?}", queue.pop_first().map(|t| t.name));
    let more = BTreeSet::from([task(3, 1, "x"), task(9, 1, "y"), task(9, 2, "z")]);
    println!("{:?}", more.iter().map(|t| t.name).collect::<Vec<_>>());
    println!("{:?}", queue.union(&more).map(|t| t.id).collect::<Vec<_>>());

    println!(
        "{:?}",
        tally(&[task(5, 1, "p"), task(6, 1, "q"), task(5, 2, "r")])
            .iter()
            .map(|(t, n)| (t.name, *n))
            .collect::<Vec<_>>()
    );
    println!("{:?}", tally(&["b", "a", "b"]));
    let scores = BTreeSet::from([Score(3), Score(9), Score(3)]);
    println!("{:?}", scores);
    let sizes = BTreeMap::from([((Size::Small, 1), "s1"), ((Size::Large, 2), "l2")]);
    println!("{:?}", sizes);

    println!(
        "{} {}",
        distinct(vec![task(1, 1, ""), task(1, 2, "")]),
        distinct(vec![3, 1, 3])
    );
}
