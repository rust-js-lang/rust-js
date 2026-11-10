// A `VecDeque`'s `retain_back(len)`: its last `len` kept, those before them
// dropped from its front in their order, across both of its halves, the
// rest still dropped after one's drop panics, as std's `drop_in_place` of
// each half goes on (#1277).
use std::collections::VecDeque;
use std::panic::{catch_unwind, AssertUnwindSafe};

#[derive(Debug)]
struct D(u32);
impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
        if self.0 == 13 {
            panic!("drop {}", self.0);
        }
    }
}

fn main() {
    let mut q: VecDeque<D> = (1..6).map(D).collect();
    q.retain_back(3);
    println!("{q:?}");
    q.retain_back(9);
    println!("{q:?}");
    // Both halves: items pushed at its front wrap around.
    let mut w: VecDeque<D> = VecDeque::with_capacity(4);
    w.push_back(D(1));
    w.push_back(D(2));
    w.push_front(D(0));
    w.push_front(D(9));
    w.retain_back(1);
    println!("{w:?}");
    let mut p: VecDeque<D> = (11..16).map(D).collect();
    let r = catch_unwind(AssertUnwindSafe(|| p.retain_back(1)));
    println!("{} {p:?}", r.is_err());
    let mut n: VecDeque<u32> = (0..6).collect();
    n.retain_back(2);
    println!("{n:?}");
}
