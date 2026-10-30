// `collect()` into a `Vec` of what isn't `TrustedLen` but has no end once
// it gives an item: std's `SpecFromIter` steps it once, then asks room for
// its hint's lower bound and one more, `usize::MAX`, and panics `capacity
// overflow`, dropping the item it made. A `cycle()` of what has items
// does, as does a `skip(n)` of what has no end, through what keeps each
// item (ADR 0940, amended). A `cycle()` with nothing gives nothing. A
// `filter` before or after a `cycle()`, or a `successors` that never ends,
// has a lower bound of 0 or 1, so std asks for a little room and goes on
// until its closure stops it. A `repeat()` is `TrustedLen`, so it panics
// before any item's made.
use std::any::Any;
use std::cell::Cell;
use std::panic::{self, AssertUnwindSafe};
use std::sync::atomic::{AtomicUsize, Ordering};

struct Noisy(&'static str);

impl Drop for Noisy {
    fn drop(&mut self) {
        println!("dropped {}", self.0);
    }
}

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

// How many items have been made: a few, where std panics after the first
// of each, and a runaway one stops, not running on without end.
static MADE: AtomicUsize = AtomicUsize::new(0);

fn made(i: usize) -> u8 {
    assert!(MADE.fetch_add(1, Ordering::Relaxed) < 20, "ran on");
    println!("made {i}");
    i as u8
}

fn main() {
    let items = [1usize, 2];
    let cycled = items.iter().cycle();
    check("cycle cloned map", move || {
        cycled.cloned().map(made).collect::<Vec<u8>>()
    });
    let cycled = items.iter().cycle();
    check("cycle map", move || cycled.map(|&i| made(i)).collect::<Vec<u8>>());
    let cycled = items.iter().copied().cycle();
    check("copied cycle", move || cycled.map(made).collect::<Vec<u8>>());
    let cycled = items.iter().cycle();
    check("cycle enumerate", move || {
        cycled.enumerate().map(|(n, &i)| made(i + n)).collect::<Vec<u8>>()
    });
    let (cycled, other) = (items.iter().cycle(), items.iter().cycle());
    check("cycle zip", move || {
        cycled.zip(other).map(|(&i, _)| made(i)).collect::<Vec<u8>>()
    });
    let (cycled, other) = (items.iter().cycle(), [9].iter().cycle());
    check("cycle chain", move || {
        cycled.chain(other).map(|&i| made(i)).collect::<Vec<u8>>()
    });
    let empty: [usize; 0] = [];
    let cycled = empty.iter().cycle();
    check("cycle empty", move || cycled.cloned().map(made).collect::<Vec<u8>>());
    let held = items.iter().cycle().cloned().map(made);
    check("cycle held", move || held.collect::<Vec<u8>>());
    let endless = 0..;
    check("skip endless", move || endless.skip(2).map(made).collect::<Vec<u8>>());

    // The item it made is dropped as it unwinds.
    let endless = 0..;
    check("skip endless owned", move || {
        endless.skip(1).map(|_| Noisy("made")).collect::<Vec<Noisy>>().len()
    });

    // A lower bound of 0: std asks for a little and goes on.
    let mut seen = 0;
    let cycled = items.iter().cycle();
    check("cycle filter", move || {
        cycled
            .filter(|&&i| {
                seen += 1;
                assert!(seen < 6, "ran on");
                i == 2
            })
            .collect::<Vec<_>>()
    });
    let stepped = Cell::new(0);
    let cycled = items
        .iter()
        .filter(|&&i| {
            stepped.set(stepped.get() + 1);
            assert!(stepped.get() < 6, "ran on");
            i == 2
        })
        .cycle();
    check("filter cycle", move || cycled.collect::<Vec<_>>());
    let counted = std::iter::successors(Some(1usize), |&n| {
        assert!(n < 4, "ran on");
        Some(n + 1)
    });
    check("successors", move || counted.collect::<Vec<usize>>());

    // `TrustedLen`, of no upper bound: before any item's made.
    let repeated = std::iter::repeat(3);
    check("repeat map", move || repeated.map(made).collect::<Vec<u8>>());

    // A zip with what has an end: its lower bound, which fits.
    let cycled = items.iter().cycle();
    check("cycle zip with an end", move || {
        cycled.zip([7, 8].iter()).map(|(&i, _)| made(i)).collect::<Vec<u8>>()
    });
    // Items of no size: room for any number, so it goes on until what
    // makes them stops it.
    let cycled = items.iter().cycle();
    check("cycle units", move || {
        cycled
            .map(|&i| {
                made(i);
            })
            .collect::<Vec<()>>()
    });
}
