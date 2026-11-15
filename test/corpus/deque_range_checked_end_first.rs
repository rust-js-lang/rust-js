// A `VecDeque`'s `range` and `range_mut` are checked as std's
// `slice::range` checks them, its end first, then its start against its
// end, not as `&v[a..b]` is, its start first (#1937): `5..7` of three items
// says "range end index 7", and `4..` says it starts at 4 but ends at 3.
// Each panic's message is printed, and what an in-range one gives.
use std::any::Any;
use std::collections::VecDeque;
use std::hint::black_box;
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
        Ok(value) => println!("{what}: {value:?}"),
        Err(payload) => println!("{what}: {}", message(payload)),
    }
}

fn main() {
    let n = |x: usize| black_box(x);
    let d = VecDeque::from([1u32, 2, 3]);
    check("range 5..7", || d.range(n(5)..n(7)).count());
    check("range 5..=6", || d.range(n(5)..=n(6)).count());
    check("range 4..", || d.range(n(4)..).count());
    check("range 2..1", || d.range(n(2)..n(1)).count());
    check("range ..4", || d.range(..n(4)).count());
    check("range 1..3", || d.range(n(1)..n(3)).sum::<u32>());
    check("range 3..", || d.range(n(3)..).count());
    check("range_mut 5..7", || {
        let mut e = d.clone();
        e.range_mut(n(5)..n(7)).count()
    });
    check("range_mut 4..", || {
        let mut e = d.clone();
        e.range_mut(n(4)..).count()
    });
    check("range_mut 5..=6", || {
        let mut e = d.clone();
        e.range_mut(n(5)..=n(6)).count()
    });
    check("range_mut 1..=2", || {
        let mut e = d.clone();
        for x in e.range_mut(n(1)..=n(2)) {
            *x *= 10;
        }
        e
    });
    check("range_mut of one pushed to its front", || {
        let mut e = d.clone();
        e.push_front(0);
        for x in e.range_mut(n(0)..n(2)) {
            *x += 1;
        }
        e
    });
}
