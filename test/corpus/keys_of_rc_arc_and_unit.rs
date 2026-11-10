// A hash map's keys that are `Rc`s and `Arc`s, a string interner's
// `Rc<str>` and a struct's, found by what they point at's `Hash` and `Eq`
// (ADR 1120), each clone counted as std's is; and `()`, one key.

use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use std::sync::Arc;

#[derive(Debug, PartialEq, Eq, Hash)]
struct Point {
    x: i32,
    y: i32,
}

fn main() {
    let mut m = HashMap::new();
    let k: Rc<str> = "a".into();
    m.insert(k.clone(), 1);
    println!("{:?}", m.get(&k));
    let other: Rc<str> = "a".into();
    println!("{:?} {}", m.get(&other), Rc::strong_count(&k));
    m.insert(other.clone(), 2);
    println!("{:?} {} {}", m.get(&k), Rc::strong_count(&k), Rc::strong_count(&other));
    let removed = m.remove_entry(&other);
    println!("{:?} {} {}", removed, Rc::strong_count(&k), Rc::strong_count(&other));
    drop(removed);
    println!("{}", Rc::strong_count(&k));

    let mut s: HashSet<Rc<Point>> = HashSet::new();
    let p = Rc::new(Point { x: 1, y: 2 });
    s.insert(p.clone());
    println!("{} {}", s.insert(Rc::new(Point { x: 1, y: 2 })), Rc::strong_count(&p));
    println!("{}", s.contains(&Rc::new(Point { x: 1, y: 2 })));
    s.clear();
    println!("{}", Rc::strong_count(&p));

    let mut a: HashMap<Arc<String>, u32> = HashMap::new();
    a.insert(Arc::new("x".to_string()), 1);
    *a.entry(Arc::new("x".to_string())).or_insert(0) += 5;
    println!("{:?}", a);

    let mut u = HashMap::new();
    u.insert((), 1);
    println!("{:?}", u.get(&()));
    u.insert((), 2);
    println!("{:?} {}", u, u.len());
    let mut us: HashSet<()> = HashSet::new();
    println!("{} {} {}", us.insert(()), us.insert(()), us.len());
}
