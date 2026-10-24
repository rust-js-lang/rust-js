// A closure given to `catch_unwind` through `AssertUnwindSafe` is called
// as what it holds, `(self.0)()`, and dropped as that call returns or
// unwinds, what it took with it (ADR 0660, #196).
use std::panic::{self, AssertUnwindSafe};

struct Loud(u32);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn boom(n: u32) -> u32 {
    let _held = Loud(n);
    if n > 0 {
        panic!("boom {n}");
    }
    n
}

fn main() {
    let kept = Loud(1);
    let returned = panic::catch_unwind(AssertUnwindSafe(move || {
        let _kept = &kept;
        7
    }));
    println!("returned {}", returned.is_ok());

    let unwound = Loud(2);
    let thrown = panic::catch_unwind(AssertUnwindSafe(move || {
        let _unwound = &unwound;
        boom(3)
    }));
    println!("thrown {}", thrown.is_err());
}
