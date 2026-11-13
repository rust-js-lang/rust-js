// A library's `Drop` that asks `std::env::args()`: in the optimized MIR its
// consumer compiles the drop from, `Args` may be one zero-sized constant, a
// struct whose fields are of no size themselves on the platform, which
// rust-js crashed on (#1707). Its value is std's own iterator, no plain
// value, so where it's such a constant the consumer is refused.
pub struct Node {
    pub value: u32,
    pub next: *mut Node,
}

impl Drop for Node {
    fn drop(&mut self) {
        println!("drop node");
        if std::env::args().count() > 5 {
            panic!("drop");
        }
    }
}

pub fn sum(mut at: *const Node) -> u32 {
    let mut total = 0;
    while !at.is_null() {
        unsafe {
            total += (*at).value;
            at = (*at).next;
        }
    }
    total
}
