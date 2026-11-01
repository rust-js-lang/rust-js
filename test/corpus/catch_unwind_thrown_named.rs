// A `catch_unwind` whose arms read a name `thrown` of the program's own is
// a `try` as any other, its `catch`'s binding given apart from it (rd551):
// React's Fizz renderNode, a `let thrown;` each arm's `let Err(..) else`
// sets, a guard dropped as its arm unwinds; a parameter named `thrown`
// an arm reads; a closure named `thrown` an arm calls; a payload named
// `thrown`; a `let Err(..) else` ending its function; and a closure of
// statements whose value a `match` arm keeps in a variable.
use std::any::Any;
use std::cell::Cell;
use std::panic::{AssertUnwindSafe, catch_unwind, resume_unwind};

struct Noisy<'a>(&'a Cell<u32>, u32);

impl Drop for Noisy<'_> {
    fn drop(&mut self) {
        self.0.set(self.0.get() * 10 + self.1);
    }
}

fn message_of(payload: &(dyn Any + Send)) -> String {
    if let Some(message) = payload.downcast_ref::<&str>() {
        message.to_string()
    } else if let Some(message) = payload.downcast_ref::<String>() {
        message.clone()
    } else {
        String::from("?")
    }
}

fn fails(n: u32) -> u32 {
    if n > 2 {
        panic!("big {n}");
    }
    n
}

/// renderNode's: each arm restores first, then keeps what it caught in
/// the function's `thrown`, resumed after the `match`.
fn render_node(n: u32, log: &Cell<u32>, segment: Option<u32>) {
    let thrown;
    match segment {
        None => {
            let previous = log.get();
            let Err(thrown_value) = catch_unwind(AssertUnwindSafe(|| log.set(fails(n)))) else {
                return;
            };
            log.set(previous + 2);
            thrown = thrown_value;
            log.set(log.get() + 1);
        }
        Some(step) => {
            let _inner = Noisy(log, 3);
            let Err(thrown_value) = catch_unwind(AssertUnwindSafe(|| log.set(fails(n + step)))) else {
                return;
            };
            if message_of(&*thrown_value).len() > 5 {
                log.set(log.get() + 4);
            }
            thrown = thrown_value;
            log.set(log.get() + 5);
        }
    }
    log.set(log.get() * 10);
    resume_unwind(thrown);
}

/// A parameter named `thrown` the `Err` arm reads.
fn given(n: u32, thrown: u32) -> String {
    match catch_unwind(|| fails(n)) {
        Ok(v) => v.to_string(),
        Err(payload) => format!("{} {thrown}", message_of(&*payload)),
    }
}

/// A closure named `thrown` the `Err` arm calls.
fn called(n: u32) -> String {
    let thrown = |payload: &(dyn Any + Send)| format!("<{}>", message_of(payload));
    match catch_unwind(|| fails(n)) {
        Ok(v) => v.to_string(),
        Err(error) => thrown(&*error),
    }
}

/// The payload itself named `thrown`.
fn payload_named(n: u32) -> String {
    match catch_unwind(|| fails(n)) {
        Ok(v) => v.to_string(),
        Err(thrown) => message_of(&*thrown),
    }
}

/// A `let Err(..) else` that ends its function, resumed after a write.
fn ended(n: u32, log: &Cell<u32>, thrown: u32) {
    let Err(x) = catch_unwind(|| fails(n)) else {
        return;
    };
    log.set(log.get() + thrown);
    resume_unwind(x);
}

/// A closure of statements whose value an arm keeps, a guard restoring
/// after, as React's createChildReconciler sets `result`.
fn kept(n: u32, log: &Cell<u32>) -> u32 {
    let result;
    {
        let _restore = Noisy(log, 5);
        match catch_unwind(AssertUnwindSafe(|| {
            log.set(log.get() + 1);
            let first = fails(n);
            log.set(log.get() + 10);
            // Twice what it found.
            first * 2
        })) {
            Ok(v) => result = v,
            Err(thrown) => {
                let message = message_of(&*thrown);
                result = message.len() as u32;
            }
        }
    }
    result + 100
}

fn main() {
    for (n, segment) in [(1, None), (5, None), (1, Some(1)), (2, Some(4))] {
        let log = Cell::new(0);
        let rendered = catch_unwind(AssertUnwindSafe(|| render_node(n, &log, segment)));
        println!(
            "render_node {n} {segment:?} {:?} {}",
            rendered.map_err(|e| message_of(&*e)),
            log.get()
        );
    }
    for n in [1, 3] {
        println!("given {}", given(n, 7));
        println!("called {}", called(n));
        println!("payload_named {}", payload_named(n));
        let log = Cell::new(0);
        let ended = catch_unwind(AssertUnwindSafe(|| ended(n, &log, 9)));
        println!("ended {:?} {}", ended.map_err(|e| message_of(&*e)), log.get());
        let log = Cell::new(0);
        println!("kept {} {}", kept(n, &log), log.get());
    }
}
