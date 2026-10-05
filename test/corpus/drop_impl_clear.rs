//@ compile-fail: rust-js does not support giving `<Wrap<T> as Empty>::empty`'s `T` a type with a destructor, where it takes none
// A generic impl's `T` may have a destructor (ADR 0098), and `clear` would
// empty the `Vec` without running it: the impl takes none of one, and a
// call that gives one is refused (ADR 0190), as `clear` of a `Vec` of a
// type with one is.
trait Empty {
    fn empty(&mut self);
}

struct Wrap<T>(Vec<T>);

impl<T> Empty for Wrap<T> {
    fn empty(&mut self) {
        self.0.clear();
    }
}

struct Loud;

impl Drop for Loud {
    fn drop(&mut self) {
        println!("dropped");
    }
}

fn main() {
    let mut w = Wrap(vec![Loud]);
    w.empty();
    println!("end");
}
