//@ library-differs
// `BuildHasherDefault<DefaultHasher>`, which holds nothing, its `default()`,
// `hash_one(x)` and `build_hasher()`, std's: a `DefaultHasher` of each, as
// uuid's `test_compare` hashes its ids (#1155).
use std::hash::{BuildHasher, BuildHasherDefault, DefaultHasher, Hash, Hasher};

#[derive(Hash)]
struct Id(u64, &'static str);

fn main() {
    let built = BuildHasherDefault::<DefaultHasher>::default();
    let a = built.hash_one(Id(1, "a"));
    let b = BuildHasherDefault::<DefaultHasher>::default().hash_one(Id(1, "a"));
    let mut h = built.build_hasher();
    Id(2, "b").hash(&mut h);
    println!("{a} {} {}", a == b, h.finish());
}
