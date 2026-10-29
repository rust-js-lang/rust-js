// A closure rustc infers `Fn` or `FnMut`, as it only borrows or changes
// what it took, given where it's made to a binding that calls it once,
// `js::set_timeout(move || { let m = &n; .. }, ms)`'s, whose handler is a
// union's `Function` member called by `FnOnce` (ADR 1020): JS calls it once
// and keeps it, as Rust's `call_once` takes it, so what it took is dropped
// as that call ends, after what its own body drops, in the order it took
// them, on a `return` its body writes, and as it unwinds. One rustc infers
// `FnOnce` drops what it took in its own body, as before.
use std::panic::{self, AssertUnwindSafe};

struct Noisy(&'static str);

impl Drop for Noisy {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

/// A handler, `string | Function` as TypeScript's `TimerHandler` is: any
/// function called once.
#[cfg_attr(rust_js, rust_js::types = "string | Function")]
trait Handler {
    #[cfg(not(rust_js))]
    fn run(self);
}

impl<F: FnOnce() + 'static> Handler for F {
    #[cfg(not(rust_js))]
    fn run(self) {
        self()
    }
}

/// `handler.call()`: JS calls the handler once, where it's given, as
/// `setTimeout` does later.
#[cfg_attr(rust_js, rust_js::link_name = "call")]
#[cfg_attr(rust_js, allow(unused_variables))]
fn run_once<H: Handler>(this: H) {
    #[cfg(not(rust_js))]
    this.run();
}

fn borrowed() {
    let first = Noisy("first");
    let second = Noisy("second");
    run_once(move || {
        let s = &second;
        let f = &first;
        let local = Noisy("local");
        println!("run {} {} {}", s.0, f.0, local.0);
    });
    println!("after borrowed");
}

fn returns_early(early: bool) {
    let held = Noisy("held");
    run_once(move || {
        let h = &held;
        if early {
            println!("early {}", h.0);
            return;
        }
        println!("late {}", h.0);
    });
}

fn changed() {
    let held = Noisy("counted");
    let mut count = 0;
    run_once(move || {
        count += 1;
        let h = &held;
        println!("count {count} {}", h.0);
    });
    println!("count after {count}");
}

fn consumed() {
    let held = Noisy("consumed");
    run_once(move || {
        println!("consuming {}", held.0);
        drop(held);
        println!("consumed");
    });
}

fn unwinds() {
    let held = Noisy("unwound");
    let caught = panic::catch_unwind(AssertUnwindSafe(|| {
        run_once(move || {
            let h = &held;
            let _local = Noisy("unwound local");
            if !h.0.is_empty() {
                panic!("boom {}", h.0);
            }
            println!("not reached");
        });
    }));
    println!("caught {}", caught.is_err());
}

fn unwinds_bare() {
    let held = Noisy("unwound bare");
    let caught = panic::catch_unwind(AssertUnwindSafe(|| {
        run_once(move || {
            let h = &held;
            if !h.0.is_empty() {
                panic!("bare {}", h.0);
            }
        });
    }));
    println!("caught bare {}", caught.is_err());
}

fn main() {
    borrowed();
    returns_early(true);
    returns_early(false);
    changed();
    consumed();
    unwinds();
    unwinds_bare();
}
