//@ compile-fail: `clone_from_slice` of items whose `clone_from` may be the crate's
// `clone_from_slice` of `Reverse`s whose value's `clone_from` is the
// crate's own: std's
// `clone_from` of one is its value's, which rust-js doesn't write of a
// `Reverse` yet (#927), so refused, never `*dest = source.clone()` (#928).
use std::cmp::Reverse;

struct Counted(u32);

impl Clone for Counted {
    fn clone(&self) -> Counted {
        Counted(self.0)
    }

    fn clone_from(&mut self, source: &Counted) {
        println!("clone_from {} <- {}", self.0, source.0);
        self.0 = source.0;
    }
}

fn main() {
    let mut v = [Reverse(Counted(3))];
    v.clone_from_slice(&[Reverse(Counted(4))]);
    println!("{}", (v[0].0).0);
}
