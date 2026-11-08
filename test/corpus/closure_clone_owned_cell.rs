// A clone of a closure that took a `Cell` it owns through a `Box` or a
// `Vec` has a cell of its own, as cloning the box or the vector copies it:
// cloned apart, `$cloning`'s, as one that took the `Cell` itself is (ADR
// 0522); one that took an `Rc` of it shares it, the closure itself (#989).
use std::cell::Cell;
use std::rc::Rc;

fn main() {
    let boxed = Box::new(Cell::new(0));
    let counter = move || {
        boxed.set(boxed.get() + 1);
        boxed.get()
    };
    let counter2 = counter.clone();
    println!("{} {} {}", counter(), counter(), counter2());

    let cells = vec![Cell::new(10)];
    let bump = move || {
        cells[0].set(cells[0].get() + 1);
        cells[0].get()
    };
    let bump2 = bump.clone();
    println!("{} {}", bump(), bump2());

    let shared = Rc::new(Cell::new(0));
    let count = move || {
        shared.set(shared.get() + 1);
        shared.get()
    };
    let count2 = count.clone();
    println!("{} {}", count(), count2());
}
