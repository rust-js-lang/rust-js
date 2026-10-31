// A JS `Map` of `Option` values holds a `None` as `null`, TypeScript's
// `Map<K, V | null>`, `#[rust_js::nullable(V)]` of the type, as
// the js crate's `Map` and `WeakMap` are (ADR 0884, amended, the
// user's decision): `set(key, None)` is `map.set(key, null)`, and `get`,
// where its value is held where it's made, is JS's read itself, `undefined`
// of a key it lacks, `null` of one whose value is `None` (ADR 1017), as
// React's Fizz runtime reads it, `map.get(name) === null`. Where it isn't,
// given to a function or shown, it's `$someGet`'s box. A value that may
// look like `None` itself, an `Option<Option<u32>>`, keeps its box. The
// binding's native body keeps its keys and values in order, as JS's does.

use std::cell::RefCell;
use std::fmt::Debug;

/// A JS `Map`, as the js crate's is: its keys and values, in the order
/// they were set.
#[cfg_attr(rust_js, rust_js::types = "Map<K, V>")]
#[cfg_attr(rust_js, rust_js::nullable(V))]
struct Map<K: 'static, V: 'static>(RefCell<Vec<(K, V)>>);

impl<K: PartialEq + Copy, V: Copy> Map<K, V> {
    /// `map.get(key)`: its value of `key`, `None` of a key it lacks.
    #[cfg_attr(rust_js, rust_js::link_name = "get")]
    fn get(&self, key: K) -> Option<V> {
        let entries = self.0.borrow();
        entries.iter().find(|(k, _)| *k == key).map(|&(_, value)| value)
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

#[derive(Debug, PartialEq)]
struct Element {
    name: &'static str,
}

/// What `map` has of `key`, of any value: its box.
fn read<V: Copy + Debug>(map: &Map<&'static str, V>, key: &'static str) -> String {
    match map.get(key) {
        Some(value) => format!("{value:?}"),
        None => "absent".to_string(),
    }
}

/// React's Fizz runtime's pairing of view transitions: each claimed one
/// set `None`, and one never claimed told by `== Some(None)`.
fn pair(names: &[&'static str], exits: &[&'static str]) {
    let appearing: &Map<&str, Option<&Element>> = new_map();
    for &name in names {
        appearing.set(name, Some(Box::leak(Box::new(Element { name }))));
    }
    for &exit in exits {
        let paired = appearing.get(exit).flatten();
        if let Some(paired) = paired {
            println!("{exit} pairs with {}", paired.name);
            appearing.set(exit, None);
        }
    }
    for &name in names {
        let claimed = appearing.get(name) == Some(None);
        println!("{name} claimed {claimed}");
    }
    // A `None` read, `null`, is the `None` the program writes, `undefined`.
    let none: Option<&Element> = None;
    for &name in names {
        let value = appearing.get(name).flatten();
        println!("{name} none {} {}", value == none, value.is_none());
    }
    let missing = appearing.get("missing") == Some(None);
    println!("missing claimed {missing}");
    appearing.for_each(|value, key| println!("{key} = {:?} {}", value.map(|e| e.name), value == None));
    println!("{} {}", read(appearing, "a"), read(appearing, "missing"));
}

fn main() {
    pair(&["a", "b", "c"], &["b", "x", "c"]);

    let counts: &Map<&str, Option<u32>> = new_map();
    counts.set("none", None);
    counts.set("zero", Some(0));
    for key in ["none", "zero", "absent"] {
        let read = match counts.get(key) {
            None => "absent".to_string(),
            Some(None) => "none".to_string(),
            Some(Some(n)) => n.to_string(),
        };
        println!("{key} is {read}");
        println!("{key}: {:?}", counts.get(key));
    }
    if let Some(Some(n)) = counts.get("zero") {
        println!("zero is {n}");
    }
    println!("{}", counts.get("none").is_some());
    println!("{} {} {}", read(counts, "none"), read(counts, "zero"), read(counts, "absent"));

    let nested: &Map<&str, Option<Option<u32>>> = new_map();
    nested.set("some none", Some(None));
    nested.set("none", None);
    nested.set("three", Some(Some(3)));
    for key in ["some none", "none", "three", "absent"] {
        println!("{key}: {:?} {}", nested.get(key), read(nested, key));
    }
}
