// Room hashbrown checks for a hash table's entries whose layout a type
// parameter gives, by their size and alignment (ADR 0774, amended): of a
// generic `HashMap`'s or `HashSet`'s `with_capacity` and `reserve`, and a
// collect into one, through a `B: FromIterator` too, whose dictionary is
// made here (#1450), or an `extend` of one by a count the program gives, the
// layout its caller gives, `KVSize` and `KVAlign`, a set's `TSize` and
// `TAlign`, so a count past `isize::MAX` bytes of its table panics `Hash
// table capacity overflow` as native Rust does, on wasm32 too, and one
// within it doesn't (#1437).
use std::any::Any;
use std::collections::{HashMap, HashSet};
use std::hash::Hash;
use std::panic::{self, AssertUnwindSafe};

fn message(payload: Box<dyn Any + Send>) -> String {
    if let Some(s) = payload.downcast_ref::<&str>() {
        s.to_string()
    } else if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else {
        "another payload".to_string()
    }
}

fn check<R>(what: &str, f: impl FnOnce() -> R) {
    match panic::catch_unwind(AssertUnwindSafe(f)) {
        Ok(_) => println!("{what}: ok"),
        Err(payload) => println!("{what}: {}", message(payload)),
    }
}

fn map_with_capacity<K, V>(n: usize) -> usize {
    HashMap::<K, V>::with_capacity(n).len()
}

fn map_reserve<K: Hash + Eq, V>(k: K, v: V, n: usize) -> usize {
    let mut m = HashMap::new();
    m.insert(k, v);
    m.reserve(n);
    m.len()
}

fn set_with_capacity<T>(n: usize) -> usize {
    HashSet::<T>::with_capacity(n).len()
}

fn set_reserve<T: Hash + Eq>(x: T, n: usize) -> usize {
    let mut s = HashSet::new();
    s.insert(x);
    s.reserve(n);
    s.len()
}

fn collected<V: Copy>(v: V, n: usize) -> usize {
    (0..n).map(|i| (i, v)).collect::<HashMap<usize, V>>().len()
}

fn gathered<I: Iterator, B: FromIterator<I::Item>>(it: I) -> B {
    it.collect()
}

fn collected_through<V: Copy>(v: V, n: usize) -> usize {
    gathered::<_, HashMap<usize, V>>((0..n).map(|i| (i, v))).len()
}

fn extended<V: Copy>(v: V, n: usize) -> usize {
    let mut m = HashMap::new();
    m.extend((0..n).map(|i| (i, v)));
    m.len()
}

fn main() {
    let past = usize::MAX / 64;
    let w = [0u64; 4];
    check("map with_capacity", || map_with_capacity::<[u64; 4], u8>(past));
    check("map with_capacity fits", || map_with_capacity::<[u64; 4], u8>(3));
    check("map reserve", || map_reserve(w, 1u8, past));
    check("map reserve fits", || map_reserve(w, 1u8, 3));
    check("set with_capacity", || set_with_capacity::<[u64; 4]>(past));
    check("set reserve", || set_reserve(w, past));
    check("set reserve fits", || set_reserve(w, 3));
    check("collect", || collected(w, past));
    check("collect fits", || collected(w, 3));
    check("collect through a bound", || collected_through(w, past));
    check("collect through a bound fits", || collected_through(w, 3));
    check("extend", || extended(w, past));
    check("extend fits", || extended(w, 3));
}
