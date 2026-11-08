//@ run-abort
// A collect whose closure panics drops what's left of its source, then
// what it collected, each as its own frame unwinds: where a drop of what's
// left panics, the process aborts before what it collected is dropped
// (#862), as Rust's cleanup does.
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
        let moved: Vec<Noisy> = items.into_iter().rev().map(|n| Noisy(n.0 + 100)).collect();
        println!("collected {}", moved.len());
    });
    println!("never {}", r.is_ok());
}
