// A JS `Map`'s value is any value, an `Option` too, as a `Dict`'s is (ADR
// 0884): `get` is `Some(None)` of a key whose value is `None`, apart from
// `None` of a key it lacks, as its `has` tells, where JS's `get` alone
// gives `undefined` of each; and what's iterated is each value as it was
// set. Of a value of its own, an `Option` of an `Option` and a `()`, and
// in a function generic over the value. The binding's native body keeps
// its keys and values in order, as JS's `Map` does.

use std::cell::RefCell;
use std::fmt::Debug;

/// A JS `Map`, as the js crate's is: its keys and values, in the order
/// they were set.
#[cfg_attr(rust_js, rust_js::types = "Map<K, V>")]
struct Map<K: 'static, V: 'static>(RefCell<Vec<(K, V)>>);

impl<K: PartialEq + Copy, V: Copy> Map<K, V> {
    /// `map.get(key)`: its value of `key`, `None` of a key it lacks.
    #[cfg_attr(rust_js, rust_js::link_name = "get")]
    fn get(&self, key: K) -> Option<V> {
        let entries = self.0.borrow();
        entries.iter().find(|(k, _)| *k == key).map(|&(_, value)| value)
    }

    /// `map.has(key)`: whether it has a value of `key`.
    #[cfg_attr(rust_js, rust_js::link_name = "has")]
    fn has(&self, key: K) -> bool {
        self.0.borrow().iter().any(|(k, _)| *k == key)
    }

    /// `map.set(key, value)`: `value` its value of `key`, where `key` was.
    #[cfg_attr(rust_js, rust_js::link_name = "set")]
    fn set(&self, key: K, value: V) {
        let mut entries = self.0.borrow_mut();
        match entries.iter_mut().find(|(k, _)| *k == key) {
            Some(entry) => entry.1 = value,
            None => entries.push((key, value)),
        }
    }

    /// `map.delete(key)`: `key` and its value gone; whether it had them.
    #[cfg_attr(rust_js, rust_js::link_name = "delete")]
    fn delete(&self, key: K) -> bool {
        let mut entries = self.0.borrow_mut();
        let before = entries.len();
        entries.retain(|(k, _)| *k != key);
        entries.len() != before
    }

    /// `map.forEach(f)`: `f` of each value and its key, in order.
    #[cfg_attr(rust_js, rust_js::link_name = "forEach")]
    fn for_each(&self, mut f: impl FnMut(V, K) + 'static) {
        for &(key, value) in self.0.borrow().iter() {
            f(value, key);
        }
    }
}

/// `new Map()`: one of nothing.
#[cfg_attr(rust_js, rust_js::link_name = "new Map")]
fn new_map<K, V>() -> &'static Map<K, V> {
    Box::leak(Box::new(Map(RefCell::new(Vec::new()))))
}

fn show(map: &Map<&'static str, Option<u32>>, key: &'static str) {
    println!("{key}: {:?} {}", map.get(key), map.has(key));
}

/// What `map` has of `key`, of any value.
fn read<V: Copy + Debug>(map: &Map<u32, V>, key: u32) -> String {
    match map.get(key) {
        Some(value) => format!("{value:?}"),
        None => "absent".to_string(),
    }
}

fn main() {
    let counts: &Map<&str, Option<u32>> = new_map();
    counts.set("none", None);
    counts.set("zero", Some(0));
    counts.set("one", Some(1));
    for key in ["none", "zero", "one", "absent"] {
        show(counts, key);
    }
    counts.set("one", None);
    show(counts, "one");
    println!("deleted {}", counts.delete("none"));
    show(counts, "none");
    counts.for_each(|value, key| println!("{key} = {value:?}"));
    for key in ["zero", "one", "absent"] {
        let read = match counts.get(key) {
            None => "absent".to_string(),
            Some(None) => "none".to_string(),
            Some(Some(n)) => n.to_string(),
        };
        println!("{key} is {read}");
    }
    if let Some(value) = counts.get("one") {
        println!("one is there, {value:?}");
    }

    let names: &Map<u32, Option<&str>> = new_map();
    names.set(1, Some(""));
    names.set(2, None);
    println!("{:?} {:?} {:?}", names.get(1), names.get(2), names.get(3));
    println!("{} {} {}", read(names, 1), read(names, 2), read(names, 3));

    let nested: &Map<u32, Option<Option<u32>>> = new_map();
    nested.set(1, Some(None));
    nested.set(2, None);
    nested.set(3, Some(Some(3)));
    for key in 1..=4 {
        println!("{key}: {:?} {}", nested.get(key), read(nested, key));
    }
    nested.for_each(|value, key| println!("{key} = {value:?}"));

    let seen: &Map<&str, ()> = new_map();
    seen.set("a", ());
    println!("{:?} {:?}", seen.get("a"), seen.get("b"));

    let plain: &Map<u32, u32> = new_map();
    plain.set(1, 0);
    println!("{:?} {:?} {} {}", plain.get(1), plain.get(2), read(plain, 1), read(plain, 2));
}
