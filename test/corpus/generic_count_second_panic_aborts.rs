//@ run-abort
// Generic code's `count` of a `Vec`'s `IntoIter` is std's, its `len()`,
// then it's dropped as one slice: where an item's drop panics, the next
// whose drop panics aborts, and none after it is dropped (#862).
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

fn how_many<I: Iterator>(items: I) -> usize {
    items.count()
}

fn main() {
    let r = panic::catch_unwind(|| {
        let items: Vec<Noisy> = (1..=5).map(Noisy).collect();
        println!("count {}", how_many(items.into_iter()));
    });
    println!("never {}", r.is_ok());
}
