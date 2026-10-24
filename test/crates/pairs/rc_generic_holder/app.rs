//! A value with a destructor in a library's generic `Rc`: dropped by the
//! last of them, as Rust does (ADRs 0494, 0844).
use std::rc::Rc;

struct Loud(u32);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop loud {}", self.0);
    }
}

pub fn main() {
    let first = dep::Holder {
        rc: Rc::new(Loud(1)),
    };
    let second = dep::Holder {
        rc: Rc::clone(&first.rc),
    };
    dep::release(first);
    println!("one left");
    dep::release(second);
    println!("end");
}
