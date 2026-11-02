// A closure of statements whose value a `match` of `catch_unwind` keeps in
// a variable declared before it sets that variable where the value is made
// (rd534): React's throwAndUnwindWorkLoop, `didFatal = throwException(..)`
// in the `try`, not a `const didFatal$1` of its own copied at its end.
// The `catch` setting it first too. Where what follows the value may panic
// and the `catch` reads the variable, or goes on to what reads it, it
// sees the variable as it was; where what follows reads it, or makes a
// closure that reads the value, the value isn't the variable's: the copy
// stays.
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::panic::{AssertUnwindSafe, catch_unwind, resume_unwind};

thread_local! {
    static WORK: Cell<Option<u32>> = const { Cell::new(None) };
    static LOG: Cell<u32> = const { Cell::new(0) };
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

fn note(n: u32) {
    LOG.set(LOG.get() * 10 + n);
}

fn throw_exception(n: u32) -> bool {
    note(1);
    if n == 0 {
        panic!("thrown {n}");
    }
    n > 5
}

fn panic_on_root_error(n: u32) {
    note(2);
    if n == 9 {
        panic!("root {n}");
    }
}

/// throwAndUnwindWorkLoop's: the value made, what may panic after it,
/// then the value; the `catch` never goes on.
fn unwind(n: u32, return_fiber: Option<u32>) {
    let did_fatal = match catch_unwind(AssertUnwindSafe(|| {
        // Find the nearest boundary.
        let did_fatal = throw_exception(n);
        if did_fatal {
            panic_on_root_error(n);
        }
        // What it found.
        did_fatal
    })) {
        Ok(did_fatal) => did_fatal,
        Err(error) => {
            if let Some(return_fiber) = return_fiber {
                WORK.set(Some(return_fiber));
                resume_unwind(error);
            } else {
                note(3);
                return;
            }
        }
    };
    if did_fatal {
        note(4);
        return;
    }
    note(5);
}

/// The `catch` sets the variable before anything reads it.
fn fallback(n: u32) -> u32 {
    let value = match catch_unwind(|| {
        let made = n * 3;
        if n > 2 {
            panic!("big {n}");
        }
        made
    }) {
        Ok(made) => made,
        Err(_) => {
            note(7);
            100
        }
    };
    value + 1
}

/// The variable had a value, which the `catch` reads: what follows the
/// value panics before the variable is set, so it's the old one.
fn kept(n: u32) -> u32 {
    let mut done = 7;
    match catch_unwind(|| {
        let found = n + 1;
        if n > 2 {
            panic!("big {n}");
        }
        found
    }) {
        Ok(found) => done = found,
        Err(_) => done += 10,
    }
    done
}

/// The variable had a value, which what follows the `match` reads where
/// the `catch` goes on: the old one where what follows the value panics.
fn kept_after(n: u32) -> u32 {
    let mut done = 7;
    match catch_unwind(|| {
        let found = n + 1;
        if n > 2 {
            panic!("big {n}");
        }
        found
    }) {
        Ok(found) => done = found,
        Err(_) => note(6),
    }
    done
}

/// The `catch` reads the variable as it leaves.
fn returned(n: u32) -> u32 {
    let mut done = 7;
    match catch_unwind(|| {
        let found = n + 1;
        if n > 2 {
            panic!("big {n}");
        }
        found
    }) {
        Ok(found) => done = found,
        Err(_) => return done + 20,
    }
    done
}

/// The `catch` reads the variable, then sets it.
fn read_then_set(n: u32) -> u32 {
    let mut done = 7;
    match catch_unwind(|| {
        let found = n + 1;
        if n > 2 {
            panic!("big {n}");
        }
        found
    }) {
        Ok(found) => done = found,
        Err(_) => {
            note(done);
            done = 30;
        }
    }
    done
}

/// A variable declared outside the loop, which the `catch` leaves for what
/// follows it, which reads it.
fn in_loop(n: u32) -> u32 {
    let mut done = 7;
    for i in 0..n {
        note(i);
        match catch_unwind(|| {
            let found = i * 3;
            if i == 2 {
                panic!("two {i}");
            }
            found
        }) {
            Ok(found) => done = found,
            Err(_) => break,
        }
    }
    done
}

/// A closure that what follows the value makes reads the value, which it
/// keeps when the variable changes after.
fn captured(n: u32) -> u32 {
    let stored: RefCell<Option<Box<dyn Fn() -> u32>>> = RefCell::new(None);
    let Ok(mut total) = catch_unwind(AssertUnwindSafe(|| {
        let next = n * 2;
        *stored.borrow_mut() = Some(Box::new(move || next));
        next
    })) else {
        return 0;
    };
    total += 1;
    total + stored.borrow().as_ref().map_or(0, |f| f())
}

/// What follows the value reads the variable before it's set.
fn read_before(n: u32) -> u32 {
    let before = Cell::new(0);
    let mut total = n;
    match catch_unwind(AssertUnwindSafe(|| {
        let next = n * 2;
        before.set(total);
        next
    })) {
        Ok(next) => total = next,
        Err(_) => total = 0,
    }
    total + before.get()
}

fn main() {
    for (n, return_fiber) in [(1, None), (7, None), (0, None), (0, Some(4)), (9, None), (9, Some(8))] {
        LOG.set(0);
        WORK.set(None);
        let unwound = catch_unwind(|| unwind(n, return_fiber));
        println!(
            "unwind {n} {return_fiber:?} {:?} {} {:?}",
            unwound.map_err(|e| message_of(&*e)),
            LOG.get(),
            WORK.get()
        );
    }
    for n in [1, 2, 3, 8] {
        println!("fallback {n} {}", fallback(n));
        println!("kept {n} {}", kept(n));
        println!("kept_after {n} {}", kept_after(n));
        println!("returned {n} {}", returned(n));
        LOG.set(0);
        println!("read_then_set {n} {} {}", read_then_set(n), LOG.get());
        println!("in_loop {n} {}", in_loop(n));
        println!("captured {n} {}", captured(n));
        println!("read_before {n} {}", read_before(n));
    }
}
