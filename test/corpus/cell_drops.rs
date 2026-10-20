// A `Cell` or a `OnceCell` of a value with a destructor: a `Cell`'s `set`
// drops the old value once the new is in its place, and each drops what it
// holds when it's dropped, a `OnceCell` only once it's set; a `OnceCell`'s
// refused `set` gives the value back (ADR 0519).
use std::cell::{Cell, OnceCell};
struct Loud(u8);
impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}
fn main() {
    let c = Cell::new(Some(Loud(1)));
    c.set(Some(Loud(2)));
    let old = c.replace(None);
    println!("old {:?}", old.as_ref().map(|l| l.0));
    let taken = c.take();
    println!("taken {}", taken.is_none());
    c.set(Some(Loud(3)));
    let o: OnceCell<Loud> = OnceCell::new();
    println!("{}", o.get().is_none());
    let _ = o.set(Loud(4));
    let again = o.set(Loud(5));
    println!("again {:?}", again.err().map(|l| l.0));
    println!("got {:?}", o.get().map(|l| l.0));
    let lazy: OnceCell<Loud> = OnceCell::new();
    println!("init {}", lazy.get_or_init(|| Loud(6)).0);
    let mut t: OnceCell<Loud> = OnceCell::new();
    let _ = t.set(Loud(7));
    println!("took {:?}", t.take().map(|l| l.0));
    println!("end");
}
