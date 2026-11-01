//@ run-fail: called `Option::unwrap()` on a `None` value
// A `Cell<Option<T>>` set to `None`, then read back by the next statement:
// its `unwrap` is Rust's check, which panics, where one of a `Some` just
// set is the value (rd331e, `cell_set_then_read_back`).
use std::cell::Cell;

fn main() {
    let canvas: Cell<Option<&str>> = Cell::new(Some("canvas"));
    println!("{:?}", canvas.get());
    canvas.set(None);
    let name = canvas.get().unwrap();
    println!("{name}");
}
