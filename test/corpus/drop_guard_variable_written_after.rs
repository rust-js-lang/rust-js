// A guard's field is a copy of the variable it was made of: written after
// the guard's made, `previous += 100`, the variable no longer holds what
// the field does, so the restore reads the field's own copy, never the
// variable (ADR 0705, ADR 1150).
use std::cell::Cell;

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

fn bump(n: u32) -> u32 {
    let mut previous = DEPTH.replace(n);
    let _restore = Restore { previous };
    previous += 100;
    println!("bumped {previous} at {}", DEPTH.get());
    previous
}

fn main() {
    DEPTH.set(1);
    println!("{}", bump(2));
    println!("{}", bump(3));
    println!("left at {}", DEPTH.get());
}
