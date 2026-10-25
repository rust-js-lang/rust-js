// A `match` of `catch_unwind` whose `Ok` arms test what it gave, React's
// `startGestureTransition`'s `Ok(Some(finish)) => return finish, Ok(None)
// => {}`, is a `try` that returns inside and a `catch` (ADR 0660,
// amended): its value tested in the `try`, as nothing there can throw;
// each as Rust runs it, a guard's drop after, through a panic too.
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
        println!("restored {}", self.previous);
        DEPTH.set(self.previous);
    }
}

fn pick(n: u32) -> Option<u32> {
    if n > 5 {
        panic!("big {n}");
    }
    if n % 2 == 0 { Some(n * 10) } else { None }
}

fn finish(n: u32) -> u32 {
    let previous = DEPTH.get();
    DEPTH.set(previous + 1);
    let _restore = Restore { previous };
    match catch_unwind(AssertUnwindSafe(|| {
        let picked = pick(n);
        picked.map(|picked| picked + 1)
    })) {
        Ok(Some(picked)) => return picked,
        Ok(None) => {}
        Err(error) => println!("caught {:?}", error.downcast_ref::<String>()),
    }
    7
}

/// Its value bound and tested further, `Ok(Some(v)) if ..`'s arm.
fn guarded(n: u32, limit: u32) -> u32 {
    match catch_unwind(|| pick(n)) {
        Ok(Some(v)) if v < limit => v,
        Ok(_) => 1,
        Err(_) => 2,
    }
}

/// An arm that may panic itself, which nothing here catches.
fn checked(n: u32) -> u32 {
    match catch_unwind(|| pick(n)) {
        Ok(Some(v)) => {
            if v > 30 {
                panic!("checked {v}");
            }
            v
        }
        Ok(None) => 1,
        Err(_) => 2,
    }
}

/// The `Result` itself read in its arm, not only what it holds.
fn whole(n: u32) -> String {
    let result = catch_unwind(|| pick(n));
    if let Ok(Some(v)) = &result {
        return format!("{v} {}", result.is_ok());
    }
    format!("none {}", result.is_err())
}

fn main() {
    for n in [1, 2, 4, 6] {
        println!("finish {n} {} depth {}", finish(n), DEPTH.get());
        for limit in [10, 30] {
            println!("guarded {n} {limit} {}", guarded(n, limit));
        }
        let outer = catch_unwind(|| checked(n));
        println!("checked {n} {:?}", outer.map_err(|e| e.downcast_ref::<String>().cloned()));
        println!("whole {n} {}", whole(n));
    }
}
