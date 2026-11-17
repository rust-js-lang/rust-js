//@ run-abort
// A comparison in `is_sorted_by` panics, and the item kept as `last`
// drops with a panic as that unwinds: a drop's panic during cleanup aborts,
// as Rust's does, before the iterator is closed (`$isSortedBy`'s
// `$dropsUnwinding`, #948).
use std::panic;
#[derive(Debug, PartialEq, PartialOrd)]
struct D(u32);
impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
        if self.0 == 2 {
            panic!("drop 2 panics");
        }
    }
}
fn main() {
    let r = panic::catch_unwind(|| {
        vec![D(1), D(2), D(3), D(4)].into_iter().is_sorted_by(|a, b| {
            if b.0 == 3 {
                panic!("cmp");
            }
            a.0 <= b.0
        })
    });
    println!("never {}", r.is_err());
}
