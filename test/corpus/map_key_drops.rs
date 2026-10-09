//@ compile-fail: a map's key with a destructor
// A map keeps the old key `insert` is given again, and drops the new one,
// which a JS `Map` doesn't: refused (ADR 0321).

use std::collections::BTreeSet;

#[derive(PartialEq, Eq, PartialOrd, Ord)]
struct Noisy(u8);

impl Drop for Noisy {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn main() {
    let mut set = BTreeSet::new();
    set.insert(Noisy(1));
    println!("{}", set.len());
}
