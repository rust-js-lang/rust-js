//@ compile-fail: a `Clone` dictionary of `std::cmp::Reverse<Counted>` given to generic code, whose std `clone_from` is its value's
// A `Clone` dictionary of a `Reverse` whose value's `clone_from` is the
// crate's own, given to generic code: std's `clone_from` of one is its
// value's, which rust-js doesn't write of a
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

fn assign<T: Clone>(dest: &mut T, source: &T) {
    dest.clone_from(source);
}

fn main() {
    let mut w = Reverse(Counted(1));
    assign(&mut w, &Reverse(Counted(2)));
    println!("{}", (w.0).0);
}
