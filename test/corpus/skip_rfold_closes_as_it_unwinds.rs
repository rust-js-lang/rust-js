// A `skip()` folded from its back, consumed, drops what it holds as a panic
// in the fold's closure unwinds, as `rfold(self)` drops the `Skip`: of a
// reversed `Vec`'s items, through an `enumerate()`, what the skip would
// have passed over and what's left are dropped (#621).
use std::panic::catch_unwind;

struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn main() {
    let r = catch_unwind(|| {
        let ds: Vec<D> = (1..6).map(D).collect();
        ds.into_iter().rev().skip(1).enumerate().rfold(0, |n, (i, d)| {
            if d.0 == 3 {
                panic!("fold");
            }
            n + i as u32 + d.0
        })
    });
    println!("panicked {}", r.is_err());
    let ds: Vec<D> = (1..6).map(D).collect();
    let n = ds.into_iter().rev().skip(1).enumerate().rfold(0, |n, (i, d)| n * 10 + i as u32 + d.0);
    println!("ended {n}");
}
