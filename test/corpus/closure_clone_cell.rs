// A `Cell` captured by value is copied by the clone, natively, and here:
// the clone is made again of a copy of it (ADR 0522).
use std::cell::Cell;

fn main() {
    let count = Cell::new(0);
    let counter = move || {
        count.set(count.get() + 1);
        count.get()
    };
    let counter2 = counter.clone();
    println!("{} {}", counter(), counter2());
}
