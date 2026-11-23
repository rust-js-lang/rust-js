//@ run-abort
// A clone of a `Vec`'s `into_iter()` whose second item's clone panics drops
// the first's clone as it unwinds, as std's `vec::IntoIter::clone` does; that
// drop panics too, while the first unwinds, which aborts the process, though
// a `catch_unwind` would have caught the first (#2412).
use std::panic::{AssertUnwindSafe, catch_unwind};

struct P {
    n: u32,
    cloned: bool,
}

impl Clone for P {
    fn clone(&self) -> P {
        println!("clone {}", self.n);
        if self.n == 2 {
            panic!("clone of 2");
        }
        P { n: self.n, cloned: true }
    }
}

impl Drop for P {
    fn drop(&mut self) {
        println!("drop {} {}", self.n, if self.cloned { "clone" } else { "given" });
        if self.cloned {
            panic!("drop of a clone");
        }
    }
}

fn main() {
    let it = vec![P { n: 1, cloned: false }, P { n: 2, cloned: false }].into_iter();
    let r = catch_unwind(AssertUnwindSafe(|| {
        let copy = it.clone();
        println!("cloned {}", copy.len());
    }));
    println!("not here {}", r.is_err());
}
