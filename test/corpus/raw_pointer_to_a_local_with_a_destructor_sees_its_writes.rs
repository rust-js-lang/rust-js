// A raw pointer to a local whose `Drop` is inlined where it's dropped, a
// node a list links to: a write to a field of the local after the pointer
// is taken is what the pointer reads, not a write nothing reads, as the
// local is used whole too, given to the pointer (#1704).
use std::ptr;

struct Node {
    value: u32,
    next: *mut Node,
}

impl Drop for Node {
    fn drop(&mut self) {
        println!("drop node");
        if std::env::args().count() > 5 {
            panic!("drop");
        }
    }
}

fn sum(mut at: *const Node) -> u32 {
    let mut total = 0;
    while !at.is_null() {
        unsafe {
            total += (*at).value;
            at = (*at).next;
        }
    }
    total
}

fn main() {
    let mut third = Node {
        value: 3,
        next: ptr::null_mut(),
    };
    let mut second = Node {
        value: 2,
        next: &mut third,
    };
    let first = Node {
        value: 1,
        next: &mut second,
    };
    println!("{}", sum(&first));
    third.value = 30;
    println!("{}", sum(&first));
}
