//! A closure rustc infers `Fn`, holding a value with a destructor, in the
//! library's struct, lent through the app's generic code to the library's,
//! which can't move it out of the reference: called by `&` alone, it's
//! dropped with the struct, here; and a reference to it given whole, to
//! the library's impl of a method that takes `self` (#2609).
use dep::{Consume, Held, Peek, peek_twice};

fn lend<R: Peek>(r: &R) -> u32 {
    peek_twice(r) + r.consume()
}

struct Loud(&'static str);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

pub fn main() {
    let d = Loud("lent to a library");
    let f = move || {
        let r = &d;
        println!("in {}", r.0);
        2
    };
    let held = Held(f);
    println!("{}", lend(&held));
    println!("end");
}
