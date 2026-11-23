// A clone of a `Vec`'s `into_iter()` whose item's clone panics drops the
// items it cloned before it, as it unwinds, as std's `vec::IntoIter::clone`
// does, an array's clone's (#2412); and where one of those drops panics too,
// a panic while one unwinds, the program aborts, as native Rust's does.
use std::panic::{AssertUnwindSafe, catch_unwind};

struct P(u32);

impl Clone for P {
    fn clone(&self) -> P {
        if self.0 == 2 {
            panic!("clone of 2");
        }
        P(self.0 + 10)
    }
}

impl Drop for P {
    fn drop(&mut self) {
        println!("drop p {}", self.0);
    }
}

fn main() {
    let it = vec![P(1), P(2), P(3)].into_iter();
    let r = catch_unwind(AssertUnwindSafe(|| {
        let copy = it.clone();
        println!("cloned {}", copy.len());
    }));
    println!("into_iter clone {}", r.is_err());
    drop(it);
    let v = vec![P(1), P(2), P(3)];
    let r = catch_unwind(AssertUnwindSafe(|| {
        let copy = v.clone();
        println!("cloned {}", copy.len());
    }));
    println!("vec clone {}", r.is_err());
    drop(v);
    println!("end");
}
