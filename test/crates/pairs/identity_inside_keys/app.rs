//! A consumer keying its own sets and maps by a library's key holding a
//! fiber, and by a tuple of one, and adding to the library's map: two
//! fibers alike are two keys, as the derived `Hash` and `Eq` find them.
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use dep::{Fiber, Key, key};

pub fn main() {
    let a = Rc::new(Fiber { tag: 1 });
    let b = Rc::new(Fiber { tag: 1 });
    let mut m: HashMap<Key, &str> = HashMap::new();
    m.insert(key(&a, 1), "a");
    m.insert(key(&b, 1), "b");
    println!("{} {:?} {:?}", m.len(), m.get(&key(&a, 1)), m.get(&key(&b, 1)));
    let pairs: HashSet<(Rc<Fiber>, u32)> = [(Rc::clone(&a), 1), (Rc::clone(&b), 1), (Rc::clone(&a), 1)]
        .into_iter()
        .collect();
    println!("{}", pairs.len());
    let mut lanes = dep::lanes(&[Rc::clone(&a), Rc::clone(&b), Rc::clone(&a)]);
    println!(
        "{} {:?} {:?}",
        lanes.len(),
        lanes.get(&key(&a, 1)),
        lanes.get(&key(&b, 1))
    );
    lanes.insert(key(&b, 2), 7);
    println!("{} {} {}", lanes.len(), lanes.contains_key(&key(&b, 2)), a.tag + b.tag);
}
