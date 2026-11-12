// An array's `map` whose closure panics part-way drops, as the panic
// unwinds, what it made so far, then the items it hasn't mapped yet, as
// std's guard of its output and its `IntoIter` of what's left do; the item
// being mapped is the closure's own (#1586). Where only one side has a
// destructor, only that side's; a map that runs drops nothing early.
use std::panic::{AssertUnwindSafe, catch_unwind};

struct Loud(u8);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {} panicking={}", self.0, std::thread::panicking());
    }
}

fn main() {
    let both = catch_unwind(|| {
        let _out = [Loud(1), Loud(2), Loud(3)].map(|l| {
            if l.0 == 2 {
                panic!("at 2");
            }
            Loud(l.0 + 10)
        });
    });
    println!("both caught {}", both.is_err());

    let made = catch_unwind(|| {
        let _out = [1u8, 2, 3].map(|n| {
            if n == 3 {
                panic!("at 3");
            }
            Loud(n + 20)
        });
    });
    println!("made caught {}", made.is_err());

    let mut seen = Vec::new();
    let left = catch_unwind(AssertUnwindSafe(|| {
        let _out = [Loud(31), Loud(32), Loud(33)].map(|l| {
            if l.0 == 31 {
                panic!("at 31");
            }
            seen.push(l.0);
            l.0
        });
    }));
    println!("left caught {} {seen:?}", left.is_err());

    let all = [Loud(41), Loud(42)].map(|l| Loud(l.0 + 1));
    println!("mapped {}", all.len());
}
