// A `catch_unwind` whose value a `match` keeps in a variable declared
// before it, beside a closure of statements, which JS declares as a
// function (rd534). Where the closure is made after the value and reads
// it, kept for a call once the variable changes, the value stays its own:
// set where it's made, the closure would read the variable's later value.
// Where a closure made before reads another variable of the same name,
// the one it shadows, the value is set where it's made: the closure reads
// that other variable however this one is set.
use std::cell::{Cell, RefCell};
use std::panic::{AssertUnwindSafe, catch_unwind};

thread_local! {
    static LOG: Cell<u32> = const { Cell::new(0) };
}

fn note(n: u32) {
    LOG.set(LOG.get() * 100 + n);
}

/// A closure of statements, made after the value, reads it later.
fn kept_by_a_closure(n: u32) -> u32 {
    let stored: RefCell<Option<Box<dyn Fn() -> u32>>> = RefCell::new(None);
    let Ok(mut total) = catch_unwind(AssertUnwindSafe(|| {
        let next = n * 2;
        let report = move || {
            let twice = next * 2;
            note(twice);
            twice + next
        };
        *stored.borrow_mut() = Some(Box::new(report));
        next
    })) else {
        return 0;
    };
    total += 1;
    total + stored.borrow().as_ref().map_or(0, |f| f())
}

/// A closure of statements reads the variable its name shadows.
fn shadowed(n: u32) -> u32 {
    let total = n + 5;
    let show = || {
        let twice = total * 2;
        note(twice);
        twice
    };
    let total = match catch_unwind(|| {
        let found = n * 3;
        if n > 2 {
            panic!("big {n}");
        }
        found
    }) {
        Ok(found) => found,
        Err(_) => 1,
    };
    total + show()
}

fn main() {
    for n in [1, 2, 3, 8] {
        LOG.set(0);
        println!("kept_by_a_closure {n} {} {}", kept_by_a_closure(n), LOG.get());
        LOG.set(0);
        println!("shadowed {n} {} {}", shadowed(n), LOG.get());
    }
}
