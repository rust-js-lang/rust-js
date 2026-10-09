// A map drops its values as Rust's does (ADR 0321): what `insert` replaces
// and `remove` takes are the caller's, and the map drops the rest, a
// `BTreeMap` in its keys' order.

use std::collections::{BTreeMap, HashMap};
use std::rc::Rc;

struct Noisy(&'static str);

impl Drop for Noisy {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn main() {
    let mut sorted = BTreeMap::new();
    sorted.insert(3, Noisy("c"));
    sorted.insert(1, Noisy("a"));
    sorted.insert(2, Noisy("b"));
    let old = sorted.insert(2, Noisy("b2"));
    println!("replaced {}", old.map(|n| n.0).unwrap_or("none"));
    let taken = sorted.remove(&1);
    println!("took {} {}", taken.is_some(), sorted.len());
    drop(taken);
    if let Some(n) = sorted.get(&3) {
        println!("has {}", n.0);
    }
    drop(sorted);
    println!("sorted dropped");

    let mut by_name: HashMap<String, Noisy> = HashMap::new();
    by_name.insert("x".to_string(), Noisy("x"));
    by_name.entry("y".to_string()).or_insert_with(|| Noisy("y"));
    println!("{} {}", by_name.len(), by_name.contains_key("y"));
    let x = by_name.remove("x");
    println!("{}", x.is_some());

    let shared = Rc::new(5);
    let mut handles = HashMap::new();
    handles.insert("a", Rc::clone(&shared));
    handles.insert("b", Rc::clone(&shared));
    println!("{}", Rc::strong_count(&shared));
    handles.remove("a");
    println!("{}", Rc::strong_count(&shared));
    drop(handles);
    println!("{}", Rc::strong_count(&shared));
}
