// `collect()` into a `Vec` of an iterator made in place whose length is a
// count the program gives, `repeat_n(x, n)`, `repeat(x).take(n)` or a
// range's `step_by(s)`: the room std's `SpecFromIter` asks first, a
// `capacity overflow` panic past `isize::MAX` bytes before any item is made,
// a `TrustedLen` one's closure never called, what it's made of made first
// (#246). And the `len()` of a range's `step_by(s)`, counted where it's
// made.
use std::any::Any;
use std::panic::{self, AssertUnwindSafe};

fn message(payload: Box<dyn Any + Send>) -> String {
    if let Some(s) = payload.downcast_ref::<&str>() {
        s.to_string()
    } else if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else {
        "another payload".to_string()
    }
}

fn check<T: std::fmt::Debug>(what: &str, f: impl FnOnce() -> T) {
    match panic::catch_unwind(AssertUnwindSafe(f)) {
        Ok(v) => println!("{what}: {v:?}"),
        Err(payload) => println!("{what}: {}", message(payload)),
    }
}

fn made() -> u8 {
    println!("made");
    7
}

fn main() {
    let all = std::hint::black_box(usize::MAX);
    check("repeat_n", || std::iter::repeat_n(1u8, all).collect::<Vec<_>>().len());
    check("repeat take", || {
        std::iter::repeat(1u8).take(all).collect::<Vec<_>>().len()
    });
    check("step_by", || (0..all).step_by(1).collect::<Vec<_>>().len());
    check("repeat_n map", || {
        std::iter::repeat_n(1u8, all)
            .map(|x| {
                println!("mapped {x}");
                x + 1
            })
            .collect::<Vec<_>>()
            .len()
    });
    check("repeat of made take", || {
        std::iter::repeat(made()).take(all).collect::<Vec<_>>().len()
    });
    let few = std::hint::black_box(3usize);
    check("repeat_n of few", || std::iter::repeat_n(1u8, few).collect::<Vec<_>>());
    check("repeat take of few", || {
        std::iter::repeat(2u8).take(few).collect::<Vec<_>>()
    });
    check("step_by of few", || (0..10).step_by(few).collect::<Vec<_>>());
    check("step_by len", || (0..10usize).step_by(few).len());
    check("step_by len past", || (12..few).step_by(2).len());
}
