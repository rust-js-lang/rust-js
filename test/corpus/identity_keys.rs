// A type equal only to itself, its `PartialEq` `ptr::eq` and its `Hash`
// `ptr::hash`, keys a set or a map by itself, as JS's `Set` and `Map` key
// an object (ADR 0864): two of equal fields are two keys, one inserted
// twice is one, and `==` is `===`; and a copy of a reference to one is the
// reference, the same key.

use std::collections::hash_map::DefaultHasher;
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};

struct Fiber {
    tag: u32,
}

impl PartialEq for Fiber {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl Eq for Fiber {}

impl Hash for Fiber {
    fn hash<H: Hasher>(&self, state: &mut H) {
        std::ptr::hash(self, state);
    }
}

fn hashed(fiber: &Fiber) -> u64 {
    let mut hasher = DefaultHasher::new();
    fiber.hash(&mut hasher);
    hasher.finish()
}

fn main() {
    let a = Fiber { tag: 1 };
    let b = Fiber { tag: 1 };
    let mut updated: HashSet<&Fiber> = HashSet::new();
    println!("{} {}", updated.insert(&a), updated.insert(&a));
    updated.insert(&b);
    println!("{} {} {}", updated.len(), updated.contains(&a), a == b);
    println!("{}", a == a);
    updated.remove(&a);
    println!("{} {}", updated.len(), updated.contains(&b));
    let mut lanes: HashMap<&Fiber, u32> = HashMap::new();
    lanes.insert(&a, 1);
    lanes.insert(&b, 2);
    *lanes.entry(&a).or_insert(0) += 10;
    println!("{:?} {:?} {}", lanes.get(&a), lanes.get(&b), a.tag + b.tag);
    println!("{} {}", hashed(&a) == hashed(&a), updated.iter().all(|f| f.tag == 1));
    updated.clear();
    println!("{}", updated.is_empty());
    // A copy of each `&Fiber` of a slice of them is the reference, the
    // same fiber, not a copy of what it points at.
    let given = [&a, &a, &b];
    let copied: HashSet<&Fiber> = given.iter().copied().collect();
    let cloned: HashSet<&Fiber> = given.iter().cloned().collect();
    println!("{} {}", copied.len(), cloned.len());
}
