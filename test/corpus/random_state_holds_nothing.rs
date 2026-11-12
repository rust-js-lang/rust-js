// A `RandomState` holds nothing JS sees: `undefined`, of `new()`,
// `Default` and `Clone`, `RandomState { .. }` as std's `Debug` shows it,
// padded or not, and in generic code too; its `build_hasher()` and
// `hash_one(x)` a `DefaultHasher`'s, whose keys are random in Rust, so no
// program sees which it hashes with (ADR 0262's "none" row). A `HashMap`'s
// or a `HashSet`'s `with_hasher`, `with_capacity_and_hasher` and `hasher()`
// of one, as its `new()` (#1420; quickcheck's `Arbitrary` of a
// `HashSet<K, H: BuildHasher + Default>`).
use std::collections::hash_map::RandomState;
use std::collections::{HashMap, HashSet};
use std::fmt::Debug;
use std::hash::{BuildHasher, Hash, Hasher};

fn made<H: BuildHasher + Default + Clone + Debug>() -> H {
    let h = H::default();
    let again = h.clone();
    println!("{again:?}");
    h
}

fn same_twice<T: Hash>(s: &RandomState, x: T) -> bool {
    let mut a = s.build_hasher();
    x.hash(&mut a);
    a.finish() == s.hash_one(&x)
}

fn collected<H: BuildHasher + Default>(n: i32) -> HashSet<i32, H> {
    (0..n).collect()
}

fn noted(s: RandomState) -> RandomState {
    println!("given");
    s
}

fn main() {
    let s = RandomState::new();
    let t = s.clone();
    let d: RandomState = Default::default();
    println!("{s:?} {t:?} {d:?}");
    println!("{s:#?}|{s:>30?}|{s:<4?}|");
    let mut h = s.build_hasher();
    h.write_u32(5);
    let mut h2 = t.build_hasher();
    h2.write_u32(5);
    println!("{}", h.finish() == h2.finish());
    println!("{}", s.hash_one(7u8) == s.hash_one(7u8));
    println!("{}", same_twice(&s, "text"));

    let g: RandomState = made();
    let set: HashSet<i32, RandomState> = collected(5);
    let mut items: Vec<_> = set.into_iter().collect();
    items.sort();
    println!("{items:?} {g:?}");

    let mut m: HashMap<&str, i32, RandomState> = HashMap::with_hasher(noted(RandomState::new()));
    m.insert("a", 1);
    *m.entry("a").or_insert(0) += 2;
    println!("{:?} {:?}", m.get("a"), m.hasher());
    let mut sized = HashMap::with_capacity_and_hasher(8, noted(s.clone()));
    sized.insert(1, "one");
    println!("{:?} {}", sized.get(&1), sized.len());
    let empty: HashSet<u8> = HashSet::with_hasher(Default::default());
    let copied = empty.hasher().clone();
    println!("{} {copied:?}", empty.is_empty());
}
