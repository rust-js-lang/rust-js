// `collect()` into a `Vec` of a `TrustedLen` with no end, each way its
// type says so: its hint, a lower bound of `usize::MAX` and no upper
// bound, is std's contract, so it panics `capacity overflow` before any
// item's made, whatever adapter keeps that hint (ADR 0940, amended).
use std::any::Any;
use std::iter::repeat;
use std::ops::RangeFrom;
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

fn check<T>(what: &str, f: impl FnOnce() -> T) {
    match panic::catch_unwind(AssertUnwindSafe(f)) {
        Ok(_) => println!("{what}: room"),
        Err(payload) => println!("{what}: {}", message(payload)),
    }
}

fn made(i: u32) -> u8 {
    println!("made {i}");
    i as u8
}

struct Holder {
    r: RangeFrom<u8>,
}

fn hidden() -> impl Iterator<Item = u8> {
    (0u32..).map(made)
}

fn main() {
    check("map", || (0..).map(made).collect::<Vec<u8>>().len());
    check("repeat", || repeat(1u8).collect::<Vec<u8>>().len());
    check("repeat map", || {
        repeat(1u32).map(made).collect::<Vec<u8>>().len()
    });
    check("zip", || {
        (0u32..).zip(repeat(1u8)).collect::<Vec<_>>().len()
    });
    check("chain", || (0u8..).chain([1u8]).collect::<Vec<_>>().len());
    check("chain after", || {
        [1u8].into_iter().chain(0u8..).collect::<Vec<_>>().len()
    });
    check("enumerate", || {
        (0u8..).enumerate().collect::<Vec<_>>().len()
    });
    check("peekable", || (0u8..).peekable().collect::<Vec<_>>().len());
    check("peeked", || {
        let mut p = (0u8..).peekable();
        p.peek();
        p.collect::<Vec<_>>().len()
    });
    check("fuse", || (0u8..).fuse().collect::<Vec<_>>().len());
    check("inspect", || {
        (0u32..)
            .inspect(|i| println!("saw {i}"))
            .collect::<Vec<_>>()
            .len()
    });
    check("copied", || repeat(&3u8).copied().collect::<Vec<_>>().len());
    check("cloned", || repeat(&3u8).cloned().collect::<Vec<_>>().len());
    check("chars", || ('a'..).collect::<Vec<char>>().len());
    check("u64", || {
        (0u64..).map(|i| i as u8).collect::<Vec<u8>>().len()
    });
    check("i8", || (0i8..).collect::<Vec<i8>>().len());
    let held = (5u32..).map(made);
    check("held", move || held.collect::<Vec<u8>>().len());
    let mut stepped = (5u32..).map(made);
    stepped.next();
    check("stepped", move || stepped.collect::<Vec<u8>>().len());
    check("zip of maps", || {
        (0u32..)
            .map(made)
            .zip((0u8..).enumerate())
            .collect::<Vec<_>>()
            .len()
    });
    check("unit", || repeat(()).map(|_| 1u8).collect::<Vec<_>>().len());
    let mut r = 3u8..;
    check("by_ref", || r.by_ref().collect::<Vec<u8>>().len());
    check("through &mut", || {
        (&mut r).map(|x| x + 1).collect::<Vec<u8>>().len()
    });
    check("opaque", || hidden().collect::<Vec<u8>>().len());
    let mut s = Holder { r: 7u8.. };
    check("field", || s.r.by_ref().collect::<Vec<u8>>().len());
    println!("{:?} {:?}", r.next(), s.r.next());
}
