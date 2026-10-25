// A unit struct's `Drop` guard holds nothing, so its drop is its body where
// it's dropped, in the `finally`, as a guard of fields is (ADR 0608,
// amended): no function of its own, nor `undefined` kept for it, as
// React's `flushActQueue` resets `isFlushing`.
use std::cell::Cell;
use std::panic::{self, AssertUnwindSafe};

thread_local! {
    static FLUSHING: Cell<bool> = const { Cell::new(false) };
}

struct FinishFlushing;

impl Drop for FinishFlushing {
    fn drop(&mut self) {
        FLUSHING.set(false);
        println!("finished");
    }
}

fn flush(items: &[u32]) -> u32 {
    let mut total = 0;
    if !FLUSHING.get() {
        FLUSHING.set(true);
        let _finish = FinishFlushing;
        if panic::catch_unwind(AssertUnwindSafe(|| {
            for &item in items {
                if item == 0 {
                    panic!("zero");
                }
                total += item;
            }
        }))
        .is_err()
        {
            println!("caught");
        }
    }
    total
}

fn main() {
    println!("{} {}", flush(&[1, 2]), FLUSHING.get());
    println!("{} {}", flush(&[1, 0, 2]), FLUSHING.get());
}
