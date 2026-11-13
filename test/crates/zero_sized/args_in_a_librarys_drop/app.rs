use dep::{Node, sum};
use std::ptr;

pub fn main() {
    let mut second = Node {
        value: 2,
        next: ptr::null_mut(),
    };
    let first = Node {
        value: 1,
        next: &mut second,
    };
    println!("{}", sum(&first));
}
