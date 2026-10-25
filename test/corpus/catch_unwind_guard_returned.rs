// React's `startGestureTransition`: a guard restores what it changed
// however the scope is left, around a `catch_unwind` whose `Ok(Some(..))`
// returns what the scope's `Option::map` gave, and a function returned
// after. The guard's `finally` and the `catch` are one `try`, `catch` and
// `finally`, the function `noop` returned after it, which the drop can't
// change (ADR 0693, amended); the mapped value tested and returned as it
// is. Each restores once: a `return` from inside, the scope's end, a panic
// caught, and one the `catch` itself throws.
use std::cell::Cell;
use std::panic::{AssertUnwindSafe, catch_unwind};

thread_local! {
    static DEPTH: Cell<u32> = const { Cell::new(0) };
}

type Finish = dyn Fn(u32) -> &'static dyn Fn() -> u32;

struct Restore {
    previous: u32,
}

impl Drop for Restore {
    fn drop(&mut self) {
        println!("restored {} (was {})", self.previous, DEPTH.get());
        DEPTH.set(self.previous);
    }
}

fn noop() -> u32 {
    0
}

fn ten() -> u32 {
    10
}

fn chosen(picked: u32) -> &'static dyn Fn() -> u32 {
    println!("chosen {picked} at depth {}", DEPTH.get());
    if picked > 2 { &ten } else { &noop }
}

fn scope(n: u32) -> u32 {
    if n == 4 {
        panic!("scope {n}");
    }
    n
}

fn report(error: Option<&String>) {
    println!("reported {error:?}");
    if error.is_some_and(|error| error.ends_with('5')) {
        panic!("report");
    }
}

fn gesture(n: u32, on_finish: Option<&Finish>) -> &'static dyn Fn() -> u32 {
    let previous = DEPTH.get();
    DEPTH.set(previous + 1);
    let _restore = Restore { previous };
    match catch_unwind(AssertUnwindSafe(|| {
        let picked = scope(n);
        on_finish.map(|on_finish| on_finish(picked))
    })) {
        Ok(Some(finish)) => return finish,
        Ok(None) => {}
        Err(error) => report(error.downcast_ref::<String>()),
    }
    &noop
}

/// Its binding's name one the scope reads, `finish`: the value keeps its
/// own, as either would be given apart, `finish$1`, to be the other's.
fn finished(n: u32, finish: Option<&Finish>) -> &'static dyn Fn() -> u32 {
    let _restore = Restore { previous: DEPTH.get() };
    match catch_unwind(AssertUnwindSafe(|| finish.map(|finish| finish(scope(n))))) {
        Ok(Some(finish)) => return finish,
        Ok(None) => {}
        Err(_) => println!("finished {n} panicked"),
    }
    &noop
}

fn main() {
    for n in [1, 3, 4, 5] {
        for finish in [None, Some(&chosen as &Finish)] {
            let given = catch_unwind(AssertUnwindSafe(|| gesture(n, finish)));
            match given {
                Ok(f) => println!("gesture {n} {} gave {}", finish.is_some(), f()),
                Err(_) => println!("gesture {n} {} panicked", finish.is_some()),
            }
            println!("depth {}", DEPTH.get());
            println!("finished {n} {} gave {}", finish.is_some(), finished(n, finish)());
        }
    }
}
