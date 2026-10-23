// What a panic unwinds through is dropped before `catch_unwind` returns
// (ADR 0660), as everywhere a panic unwinds (ADR 0098): each scope's, inside
// out, not what wasn't made yet or was moved away; what a `move` closure
// took; and a payload resumed, dropped once, where its `Err` is.
use std::panic::{self, AssertUnwindSafe};

struct Noisy(&'static str);

impl Drop for Noisy {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn deep(n: u32) {
    let _guard = Noisy("deep");
    if n > 0 {
        panic!("deep panic");
    }
}

fn main() {
    let _outer = Noisy("outer");
    let result = panic::catch_unwind(AssertUnwindSafe(|| {
        let _a = Noisy("a");
        let moved = Noisy("moved");
        let kept = Noisy("kept");
        drop(kept);
        deep(1);
        drop(moved);
        let _never = Noisy("never");
    }));
    println!("caught: {}", result.is_err());

    let held = Noisy("captured");
    let result = panic::catch_unwind(move || {
        let taken = held;
        println!("holding {}", taken.0);
        panic!("again");
    });
    println!("caught again: {}", result.is_err());

    let result = panic::catch_unwind(|| panic::resume_unwind(Box::new(Noisy("payload"))));
    println!("payload caught: {}", result.is_err());
    drop(result);
    println!("end");
}
