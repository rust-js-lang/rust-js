// `collect()` into a `Vec` of an iterator whose length is a count the
// program gives, held in a variable first, or made before an `unwrap`'s
// check or under a `RefCell` guard: the room std's `SpecFromIter` asks of
// its hint, a `capacity overflow` panic past `isize::MAX` bytes, each form's
// hint std's, a reversed range's `step_by` too, and a `step_by` of what no
// array's items bound (#1046).
use std::any::Any;
use std::cell::RefCell;
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

#[derive(Clone)]
struct Dropped(u8);

impl Drop for Dropped {
    fn drop(&mut self) {
        println!("dropped {}", self.0);
    }
}

fn hinted<I: Iterator>(it: I) -> (usize, Option<usize>) {
    it.size_hint()
}

fn main() {
    for n in [std::hint::black_box(5usize), std::hint::black_box(usize::MAX)] {
        println!("of {}", if n == 5 { "five" } else { "all" });
        check("repeat_n", || {
            let it = std::iter::repeat_n(1u8, n);
            it.collect::<Vec<_>>().len()
        });
        check("repeat take map", || {
            let it = std::iter::repeat(1u8).take(n).map(|x| x + 1);
            it.collect::<Vec<_>>()
        });
        check("range step_by", || {
            let it = (0..n).step_by(2);
            it.collect::<Vec<_>>()
        });
        check("reversed range step_by", || {
            let it = (0..n).rev().step_by(2);
            it.collect::<Vec<_>>()
        });
        check("reversed range step_by take", || {
            let it = (0..n).rev().step_by(1).take(n);
            it.collect::<Vec<_>>()
        });
        check("repeat step_by take", || {
            let it = std::iter::repeat(1u8).step_by(2).take(n);
            it.collect::<Vec<_>>()
        });
        check("repeat take step_by", || {
            let it = std::iter::repeat(1u8).take(n).step_by(2);
            it.collect::<Vec<_>>()
        });
        check("repeat_n zip", || {
            let it = std::iter::repeat_n(1u8, n).zip(0..n);
            it.collect::<Vec<_>>()
        });
        let held = Some(n);
        check("repeat_n of a dropped value after an unwrap", || {
            std::iter::repeat_n(Dropped(1), *held.as_ref().unwrap()).collect::<Vec<_>>().len()
        });
        let cell = RefCell::new(n);
        check("repeat_n of a dropped value under a guard", || {
            std::iter::repeat_n(Dropped(2), *cell.borrow()).collect::<Vec<_>>().len()
        });
    }
    println!("{:?}", hinted((0..10).rev().step_by(3)));
    println!("{:?}", hinted(('a'..='e').rev().step_by(2)));
    println!("{:?}", hinted((0..10u64).rev().step_by(4)));
    println!("{:?}", hinted((0..0).rev().step_by(4)));
}
