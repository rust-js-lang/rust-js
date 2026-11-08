//@ run-abort
// A `Vec`'s `IntoIter`'s `count` is its `len()`, then it's dropped as one
// slice: where an item's drop panics, the rest are dropped as that unwinds,
// so the next whose drop panics aborts, and none after it is dropped (#862).
use std::panic;
use std::sync::atomic::{AtomicBool, Ordering};

static ARMED: AtomicBool = AtomicBool::new(false);

struct Noisy(u32);

impl Drop for Noisy {
    fn drop(&mut self) {
        println!("drop {}", self.0);
        if self.0 == 3 {
            ARMED.store(true, Ordering::Relaxed);
            panic!("drop 3");
        }
        if ARMED.swap(false, Ordering::Relaxed) {
            panic!("drop again");
        }
    }
}

fn main() {
    let r = panic::catch_unwind(|| {
        let items: Vec<Noisy> = (1..=5).map(Noisy).collect();
        let n = items.into_iter().count();
        println!("count {n}");
    });
    println!("never {}", r.is_ok());
}
