// A generic impl's `T` may have a destructor (ADR 0098): `clear` of its
// `Vec<T>` drops each item with the drop the impl is given for `T`, as std's
// does, called directly and through its dictionary, where it was refused
// (ADR 0190).
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

fn emptied<E: Empty>(e: &mut E) {
    e.empty();
}

fn main() {
    let mut w = Wrap(vec![Loud]);
    w.empty();
    let mut again = Wrap(vec![Loud, Loud]);
    emptied(&mut again);
    println!("end");
}
