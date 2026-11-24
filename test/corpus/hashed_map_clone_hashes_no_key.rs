// A `HashMap`'s or a `HashSet`'s clone, and `clone_from`, of keys whose
// own `Hash` prints: std's table copies its buckets and asks no key's
// `hash`, so nothing prints as it's cloned; a lookup in the clone does
// (#2548). One of values that drop, whose `clone_from` drops what it held
// first, and one into a box, whose map is written in place, too.
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};

#[derive(Clone, PartialEq, Eq)]
struct Key(u32);

impl Hash for Key {
    fn hash<H: Hasher>(&self, state: &mut H) {
        println!("hash {}", self.0);
        self.0.hash(state);
    }
}

#[derive(Clone)]
struct Loud(&'static str);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn main() {
    let mut m = HashMap::new();
    m.insert(Key(1), "a");
    m.insert(Key(2), "b");
    println!("cloning");
    let cloned = m.clone();
    println!("cloned {} {:?}", cloned.len(), cloned.get(&Key(2)));
    let mut into = HashMap::new();
    into.insert(Key(9), "z");
    println!("cloning from");
    into.clone_from(&m);
    println!("cloned from {} {:?}", into.len(), into.get(&Key(1)));
    let mut held = HashMap::new();
    held.insert(Key(5), Loud("e"));
    let mut over = HashMap::new();
    over.insert(Key(7), Loud("g"));
    println!("cloning loud ones from");
    over.clone_from(&held);
    println!("cloned loud ones from {} {:?}", over.len(), over.get(&Key(5)).map(|l| l.0));
    let mut source = Box::new(HashMap::new());
    source.insert(Key(10), "p");
    let mut boxed = Box::new(HashMap::new());
    boxed.insert(Key(8), "y");
    println!("cloning into a box");
    (*boxed).clone_from(&*source);
    println!("cloned into a box {} {:?}", boxed.len(), boxed.get(&Key(10)));
    let old = std::mem::replace(&mut *boxed, HashMap::new());
    println!("replaced {} {:?} {}", old.len(), old.get(&Key(10)), boxed.len());
    let set: HashSet<Key> = [Key(3), Key(4)].into_iter().collect();
    println!("cloning a set");
    let copy = set.clone();
    println!("cloned a set {} {}", copy.len(), copy.contains(&Key(3)));
}
