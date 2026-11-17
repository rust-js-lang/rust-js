// A guard inlined where it's made restores once however its scope is
// left: its end, and a panic between the guard and the end, caught,
// after which the depth is what it was. A variable of the guard's field's
// word is declared again between, its own (ADR 1150).
use std::cell::Cell;
use std::panic::{AssertUnwindSafe, catch_unwind};

thread_local! {
    static DEPTH: Cell<u32> = const { Cell::new(0) };
}

struct Restore {
    previous: u32,
}

impl Drop for Restore {
    fn drop(&mut self) {
        println!("restored {} (was {})", self.previous, DEPTH.get());
        DEPTH.set(self.previous);
    }
}

fn enter(n: u32) -> u32 {
    let previous = DEPTH.replace(n);
    let _restore = Restore { previous };
    println!("entered {n} from {previous}");
    if n > 4 {
        let previous = n * 2;
        panic!("too deep {previous}");
    }
    n + previous
}

fn main() {
    for n in [2, 5, 3] {
        match catch_unwind(AssertUnwindSafe(|| enter(n))) {
            Ok(v) => println!("ok {v} at {}", DEPTH.get()),
            Err(_) => println!("caught at {}", DEPTH.get()),
        }
    }
}
