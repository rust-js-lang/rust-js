// A `Clone` dictionary of an `Option` whose value's `clone_from` is the
// crate's own, given to generic code: std's `clone_from` of it is its
// value's, of two `Some`s (#927), never `*dest = source.clone()` (#928).
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
    let mut w = Some(Counted(1));
    assign(&mut w, &Some(Counted(2)));
    println!("{}", w.map_or(0, |c| c.0));
}
