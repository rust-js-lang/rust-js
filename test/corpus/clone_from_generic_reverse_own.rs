// A `Clone` dictionary of a `Reverse` whose value's `clone_from` is the
// crate's own, given to generic code: std's `clone_from` of one is its
// value's, `self.0.clone_from(&source.0)`, so its dictionary's
// `clone_from` is that value's, never `*dest = source.clone()` (#928,
// #927).
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
