//@ library-differs
// A `Wrapping`'s `Hash`, std's, into a hasher generic code is given: its
// derived impl's, its field's, written into the hasher as the dictionary
// holds it, as a tuple's part is. It was refused (ADR 0463, amended; #1289).
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
