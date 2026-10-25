// A `match` of `catch_unwind` whose `Ok` arm returns is a `try` that
// returns and a `catch` of what the `Err` arm does, what follows the
// `match` after them (ADR 0660, amended): React's
// `recursivelyFlushAsyncActWork`. And a `return` in the closure, in a
// loop, where nothing runs after the `try` but its drop, is the
// function's own `return`, as React's `flushActQueue` writes it.
use std::cell::{Cell, RefCell};
use std::panic::{self, AssertUnwindSafe};

thread_local! {
    static THROWN_ERRORS: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
    static DONE: Cell<u32> = const { Cell::new(0) };
}

fn work(n: u32) {
    if n == 0 {
        panic!("zero");
    }
    DONE.set(DONE.get() + n);
}

fn flushed(n: u32) -> u32 {
    if n < 100 {
        match panic::catch_unwind(AssertUnwindSafe(|| {
            work(n);
            work(n);
        })) {
            Ok(()) => return DONE.get(),
            Err(error) => {
                let message = error.downcast_ref::<&str>().copied().unwrap_or("?");
                THROWN_ERRORS.with_borrow_mut(|errors| errors.push(message.to_string()));
            }
        }
        println!("after the match");
    }
    THROWN_ERRORS.with_borrow(|errors| errors.len() as u32)
}

struct Finish;

impl Drop for Finish {
    fn drop(&mut self) {
        println!("finish {}", DONE.get());
    }
}

fn until_seven(queue: &[u32]) {
    if !queue.is_empty() {
        let _finish = Finish;
        if let Err(_error) = panic::catch_unwind(AssertUnwindSafe(|| {
            for &n in queue {
                if n == 7 {
                    return;
                }
                work(n);
            }
            DONE.set(0);
        })) {
            DONE.set(1);
        }
    }
}

fn main() {
    println!("{}", flushed(2));
    println!("{}", flushed(0));
    println!("{}", flushed(100));
    until_seven(&[1, 7, 0]);
    until_seven(&[1, 0, 7]);
    until_seven(&[1, 2]);
    println!("{}", DONE.get());
}
