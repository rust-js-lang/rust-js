// What a closure drops as its `assert!` panics, the argument it was given,
// runs as the panic unwinds: `thread::panicking()` is `true` in its `Drop`,
// under `flat_map` as under `map`, and a drop that panics there aborts
// rather than unwinding on, as Rust's does (#1606). So too of a `Drop`
// that only notes it, which can't throw.
use std::cell::Cell;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::atomic::{AtomicBool, Ordering};

static NOTED: AtomicBool = AtomicBool::new(false);

struct Noted(Cell<bool>);

impl Drop for Noted {
    fn drop(&mut self) {
        self.0.set(true);
        NOTED.store(std::thread::panicking(), Ordering::Relaxed);
    }
}

struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {} panicking={}", self.0, std::thread::panicking());
    }
}

fn main() {
    let flat = catch_unwind(AssertUnwindSafe(|| {
        vec![D(1), D(2), D(3)]
            .into_iter()
            .flat_map(|x| {
                assert!(x.0 != 2, "flat_map");
                vec![D(x.0 + 10), x]
            })
            .fold(0, |a, d| a + d.0)
    }));
    println!("flat_map caught {}", flat.is_err());
    let mapped = catch_unwind(AssertUnwindSafe(|| {
        vec![D(1), D(2)]
            .into_iter()
            .map(|x| {
                assert!(x.0 != 2, "map");
                x
            })
            .fold(0, |a, d| a + d.0)
    }));
    println!("map caught {}", mapped.is_err());
    let noted = catch_unwind(AssertUnwindSafe(|| {
        [Noted(Cell::new(false)), Noted(Cell::new(false))]
            .into_iter()
            .enumerate()
            .flat_map(|(i, noted)| {
                assert!(i != 1, "noted");
                Some(noted)
            })
            .count()
    }));
    println!("noted caught {}, panicking {}", noted.is_err(), NOTED.load(Ordering::Relaxed));
    drop(Noted(Cell::new(false)));
    println!("dropped, panicking {}", NOTED.load(Ordering::Relaxed));
}
