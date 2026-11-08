// A cell made right there and taken or replaced in the same expression,
// `Cell::new(x).take()`, `RefCell::new(x).take()`, `Cell::new(x).replace(y)`:
// a `Cell` held as its value in JS (ADR 0287) is held in a variable of its
// own, written there, and dropped as Rust's temporary is, where its value
// was written to as if it were a place, which crashed rust-js (#958).
use std::cell::{Cell, RefCell};

#[derive(Default, Debug)]
struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn main() {
    let o = Cell::new(Some(D(1))).take();
    println!("{o:?}");
    let r = RefCell::new(D(2)).take();
    println!("{r:?}");
    let s = Cell::new(String::from("s")).replace(String::from("t"));
    println!("{s}");
    let v = Cell::new(vec![D(3)]).take();
    println!("{}", v.len());
}
