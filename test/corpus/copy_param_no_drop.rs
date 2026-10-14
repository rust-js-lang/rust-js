// A `Copy` type parameter is given no drop (ADR 0163): an impl for any
// `T: Copy` takes none, `holderShow(TCopy, TDisplay)`, though the crate
// has a destructor of its own (the second architecture review: no case
// held it, the test of it written four times).

use std::fmt::Display;

struct Loud(&'static str);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

trait Show {
    fn show(&self) -> String;
}

struct Holder<T>(T);

impl<T: Copy + Display> Show for Holder<T> {
    fn show(&self) -> String {
        let copy = self.0;
        format!("{copy}")
    }
}

fn main() {
    let _loud = Loud("a");
    println!("{}", Holder(3).show());
}
