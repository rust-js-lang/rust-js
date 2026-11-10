use dep::{A, B, Keep, by_default};
use std::rc::Rc;

pub fn main() {
    println!("{} {}", by_default(&A), by_default(&B));
    let n = Rc::new(5u32);
    println!("{} {}", A.keep(&n), B.keep(&n));
}
