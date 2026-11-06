// A generic `Drop` impl is judged for each use (#861): `Defer<F>`'s drop,
// given a closure that can't panic, never unwinds, so its guard drops in a
// `finally`; given one that may, an index past the end, it drops after the
// `try` and as a panic unwinds. A generic drop that calls a generic function
// of what it's given, which drops what it's given, and one that drops
// what's of its type parameter, are judged with their arguments too.
use std::cell::Cell;
use std::fmt::Display;
use std::panic;

thread_local! {
    static DEPTH: Cell<u32> = const { Cell::new(0) };
}

struct Defer<F: FnMut()>(F);

impl<F: FnMut()> Drop for Defer<F> {
    fn drop(&mut self) {
        (self.0)();
    }
}

struct Twice<F: FnMut()>(F);

impl<F: FnMut()> Drop for Twice<F> {
    fn drop(&mut self) {
        (self.0)();
        (self.0)();
    }
}

struct Kept(u32);

impl Drop for Kept {
    fn drop(&mut self) {
        println!("kept {}", self.0);
    }
}

fn report<T: Display, U>(value: T, kept: U) {
    println!("report {value}");
    let _kept = kept;
}

struct Reported<F: FnMut() -> u32>(F);

impl<F: FnMut() -> u32> Drop for Reported<F> {
    fn drop(&mut self) {
        report((self.0)(), Kept(1));
    }
}

struct Holder<T>(Option<T>);

impl<T> Drop for Holder<T> {
    fn drop(&mut self) {
        let held = self.0.take();
        println!("holder {}", held.is_some());
    }
}

struct Loud(&'static str);

impl Drop for Loud {
    fn drop(&mut self) {
        if self.0 == "bomb" {
            panic!("loud {}", self.0);
        }
        println!("loud {}", self.0);
    }
}

fn work(n: u32) -> u32 {
    if n > 100 {
        panic!("too big: {n}");
    }
    n + 1
}

pub fn nested(work: impl FnOnce() -> u32) -> u32 {
    let previous = DEPTH.get();
    DEPTH.set(previous + 1);
    let _restore = Defer(|| DEPTH.set(previous));
    work()
}

fn deferred(n: u32) -> u32 {
    let previous = DEPTH.get();
    let _restore = Defer(|| DEPTH.set(previous));
    DEPTH.set(n);
    work(n)
}

fn indexed(n: u32, at: usize) -> u32 {
    let items = [1, 2, 3];
    let _check = Defer(|| println!("item {}", items[at]));
    work(n)
}

fn twice(n: u32) -> u32 {
    let _twice = Twice(|| println!("twice {}", DEPTH.get()));
    work(n)
}

fn reported(n: u32) -> u32 {
    let _reported = Reported(|| n * 2);
    work(n)
}

fn held_plain(n: u32) -> u32 {
    let _holder = Holder(Some(n));
    work(n)
}

fn held_loud(n: u32, name: &'static str) -> u32 {
    let _holder = Holder(Some(Loud(name)));
    work(n)
}

fn main() {
    println!("nested {}", nested(|| DEPTH.get() * 10));
    println!("depth {}", DEPTH.get());
    let r = panic::catch_unwind(|| nested(|| panic!("inside")));
    println!("nested caught {} depth {}", r.is_err(), DEPTH.get());

    println!("deferred {}", deferred(4));
    let r = panic::catch_unwind(|| deferred(200));
    println!("deferred caught {} depth {}", r.is_err(), DEPTH.get());

    println!("indexed {}", indexed(1, 2));
    let r = panic::catch_unwind(|| indexed(1, 7));
    println!("indexed past the end caught {}", r.is_err());

    println!("twice {}", twice(2));
    let r = panic::catch_unwind(|| twice(300));
    println!("twice caught {}", r.is_err());

    println!("reported {}", reported(3));
    let r = panic::catch_unwind(|| reported(400));
    println!("reported caught {}", r.is_err());

    println!("held plain {}", held_plain(5));
    let r = panic::catch_unwind(|| held_plain(500));
    println!("held plain caught {}", r.is_err());

    println!("held loud {}", held_loud(6, "quiet"));
    let r = panic::catch_unwind(|| held_loud(7, "bomb"));
    println!("held bomb caught {}", r.is_err());
}
