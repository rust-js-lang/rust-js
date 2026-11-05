// A `skip()` of items with nothing to drop, JS's own `drop(n)`, is std's
// `Skip` as one with items to drop is: its `fold`, `try_fold`, `count`
// and `last` first ask its source's `nth(n - 1)`. Where that panics, the
// accumulator it was given is dropped as the frame unwinds, then the
// source (#735).
use std::panic::catch_unwind;

struct Acc(u32);

impl Drop for Acc {
    fn drop(&mut self) {
        println!("drop acc {}", self.0);
    }
}

struct It(u32);

impl Drop for It {
    fn drop(&mut self) {
        println!("drop it");
    }
}

impl Iterator for It {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
        self.0 += 1;
        if self.0 == 2 {
            panic!("next");
        }
        Some(self.0)
    }
}

fn main() {
    let r = catch_unwind(|| It(0).skip(2).fold(Acc(0), |a, _| a).0);
    println!("fold ok {}", r.is_ok());
    let r = catch_unwind(|| It(0).skip(2).try_fold(Acc(0), |a, _| Some(a)).map(|a| a.0));
    println!("try_fold ok {}", r.is_ok());
    let r = catch_unwind(|| It(0).skip(2).count());
    println!("count ok {}", r.is_ok());
    let r = catch_unwind(|| It(0).skip(2).last());
    println!("last ok {}", r.is_ok());
    let r = catch_unwind(|| It(0).skip(1).map(|n| n * 10).fold(Acc(1), |a, _| a).0);
    println!("mapped fold ok {}", r.is_ok());
}
