//@ compile-fail: calling `std::hash::Hash::hash`
// A `Wrapping`'s `Hash`, std's, into a hasher generic code is given: no
// dictionary carries it, as it's no type parameter's, so it's refused, not
// given the dictionary's box of a hasher (ADR 0463).
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::num::Wrapping;

struct Counted(u32);

impl Hash for Counted {
    fn hash<H: Hasher>(&self, state: &mut H) {
        Wrapping(self.0).hash(state);
    }
}

fn main() {
    let mut h = DefaultHasher::new();
    Counted(7).hash(&mut h);
    println!("{}", h.finish());
}
