// A `catch`'s payload is made where it's first read, after what its arm
// does first, and one resumed as it was given is what was thrown (rd550):
// a panic's message read after a write, then resumed; one changed through
// a function given `&mut`, or through `downcast_mut` and then moved to
// another variable, or then resumed by a closure that took it, resumed as
// what it is now; another payload resumed by a closure whose variable has
// the payload's name, as that payload; guards dropped as each
// scope unwinds; a typed payload read and resumed as itself; and a
// `catch_unwind` inside another's closure, resumed to the outer one.
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
    } else if let Some(n) = payload.downcast_ref::<i32>() {
        format!("i32 {n}")
    } else {
        String::from("?")
    }
}

fn change(payload: &mut (dyn Any + Send)) {
    if let Some(message) = payload.downcast_mut::<String>() {
        message.push('!');
    }
}

fn fails(n: u32) -> u32 {
    if n > 2 {
        panic!("big {n}");
    }
    n
}

/// What the arm writes first, then the message it reads, then resumed.
fn noted(n: u32, seen: &Cell<u32>, read: &Cell<usize>) -> u32 {
    match catch_unwind(|| fails(n)) {
        Ok(v) => v,
        Err(e) => {
            seen.set(seen.get() + 1);
            read.set(message_of(&*e).len());
            resume_unwind(e)
        }
    }
}

/// Changed before it's resumed, by a function given `&mut`.
fn changed(n: u32) -> u32 {
    match catch_unwind(|| fails(n)) {
        Ok(v) => v,
        Err(mut e) => {
            change(&mut *e);
            resume_unwind(e)
        }
    }
}

/// Changed, then moved to another variable, which is resumed.
fn moved(n: u32) -> u32 {
    match catch_unwind(|| fails(n)) {
        Ok(v) => v,
        Err(mut e) => {
            if let Some(message) = e.downcast_mut::<String>() {
                message.push('?');
            }
            let again = e;
            resume_unwind(again)
        }
    }
}

/// What `f` gives of `x`.
fn apply<T>(f: impl FnOnce(T) -> u32, x: T) -> u32 {
    f(x)
}

/// Changed, then resumed by a closure that took it.
fn captured(n: u32) -> u32 {
    match catch_unwind(|| fails(n)) {
        Ok(v) => v,
        Err(mut e) => {
            change(&mut *e);
            apply(move |()| resume_unwind(e), ())
        }
    }
}

/// Resumed by a closure that took it, as it was caught, as React's error
/// loggers rethrow in a `setTimeout`.
fn taken(n: u32) -> u32 {
    match catch_unwind(|| fails(n)) {
        Ok(v) => v,
        Err(e) => apply(move |()| resume_unwind(e), ()),
    }
}

/// Read, then another resumed by a closure whose own variable has the
/// payload's name.
fn shadowed(n: u32, seen: &Cell<usize>) -> u32 {
    match catch_unwind(|| fails(n)) {
        Ok(v) => v,
        Err(e) => {
            seen.set(message_of(&*e).len());
            let other: Box<dyn Any + Send> = Box::new(String::from("other"));
            apply(
                |x: Box<dyn Any + Send>| {
                    seen.set(seen.get() + 1);
                    let e = x;
                    resume_unwind(e)
                },
                other,
            )
        }
    }
}

/// A guard the closure holds, dropped as it unwinds, and one the arm
/// holds, dropped as the arm resumes.
fn guarded(log: &Cell<u32>) {
    if let Err(e) = catch_unwind(AssertUnwindSafe(|| {
        let _a = Noisy(log, 1);
        fails(7)
    })) {
        let _b = Noisy(log, 2);
        log.set(log.get() * 10 + 3);
        resume_unwind(e);
    }
}

/// A typed payload, read where it's first read, resumed as itself.
fn typed(n: i32, seen: &Cell<u32>) {
    let Err(e) = catch_unwind(|| resume_unwind(Box::new(n)));
    seen.set(seen.get() + 1);
    let v = e.downcast_ref::<i32>().copied().unwrap_or_default();
    seen.set(seen.get() + v as u32);
    resume_unwind(e);
}

/// An inner `catch_unwind` that writes and resumes, inside an outer one
/// that reads the message.
fn nested(log: &Cell<u32>) -> String {
    match catch_unwind(AssertUnwindSafe(|| {
        if let Err(e) = catch_unwind(AssertUnwindSafe(|| fails(log.get() + 3))) {
            log.set(log.get() + 1);
            resume_unwind(e);
        }
    })) {
        Ok(()) => String::new(),
        Err(e) => {
            log.set(log.get() + 10);
            message_of(&*e)
        }
    }
}

fn main() {
    let (seen, read) = (Cell::new(0), Cell::new(0));
    for n in [1, 5] {
        let result = catch_unwind(AssertUnwindSafe(|| noted(n, &seen, &read)));
        let message = result.as_ref().err().map(|e| message_of(&**e));
        println!("noted {:?} {} {} {message:?}", result.ok(), seen.get(), read.get());
    }
    for n in [2, 6] {
        let changed = catch_unwind(|| changed(n)).map_err(|e| message_of(&*e));
        let moved = catch_unwind(|| moved(n)).map_err(|e| message_of(&*e));
        println!("changed {changed:?} moved {moved:?}");
        let captured = catch_unwind(|| captured(n)).map_err(|e| message_of(&*e));
        let seen = Cell::new(0);
        let shadowed = catch_unwind(AssertUnwindSafe(|| shadowed(n, &seen))).map_err(|e| message_of(&*e));
        println!("captured {captured:?} shadowed {shadowed:?} {}", seen.get());
        let taken = catch_unwind(|| taken(n)).map_err(|e| message_of(&*e));
        println!("taken {taken:?}");
    }
    let log = Cell::new(0);
    let guarded = catch_unwind(AssertUnwindSafe(|| guarded(&log))).map_err(|e| message_of(&*e));
    println!("guarded {guarded:?} {}", log.get());
    let seen = Cell::new(0);
    let typed = catch_unwind(AssertUnwindSafe(|| typed(5, &seen))).map_err(|e| message_of(&*e));
    println!("typed {typed:?} {}", seen.get());
    let log = Cell::new(1);
    println!("nested {:?} {}", nested(&log), log.get());
}
