// `collect()` into a `Vec` of an iterator made in place whose length is a
// count the program gives, `repeat_n(x, n)`, `repeat(x).take(n)` or a
// range's `step_by(s)`: the room std's `SpecFromIter` asks first, a
// `capacity overflow` panic past `isize::MAX` bytes before any item is made,
// a `TrustedLen` one's closure never called, what it's made of made first
// (#246). And the `len()` of a range's `step_by(s)`, counted where it's
// made. And of a count read through a call, `*n.as_ref().unwrap()`, the
// call run once, its room asked by the iterator's hint; and of
// `repeat_n(x, n)` of a value it clones, `n`; a hint the crate writes
// asked once (#1044).
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

#[derive(Clone)]
struct Dropped(u8);

impl Drop for Dropped {
    fn drop(&mut self) {
        println!("dropped {}", self.0);
    }
}

struct Cloned(u8);

impl Clone for Cloned {
    fn clone(&self) -> Cloned {
        println!("cloned {}", self.0);
        Cloned(self.0)
    }
}

struct Up(u32);

impl Iterator for Up {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
        self.0 += 1;
        (self.0 < 6).then_some(self.0)
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        println!("hinted {}", self.0);
        (0, Some(6))
    }
}

fn counted(n: usize) -> usize {
    println!("counted");
    n
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
    let held = Some(all);
    check("repeat_n of a held count", || {
        std::iter::repeat_n(1u8, *held.as_ref().unwrap()).collect::<Vec<_>>().len()
    });
    check("repeat take of a held count", || {
        std::iter::repeat(1u8).take(*held.as_ref().unwrap()).collect::<Vec<_>>().len()
    });
    check("step_by of a held step", || {
        (0..all).step_by(*Some(1).as_ref().unwrap()).collect::<Vec<_>>().len()
    });
    check("repeat_n map of a counted count", || {
        std::iter::repeat_n(1u16, counted(all))
            .map(|x| {
                println!("mapped {x}");
                x + 1
            })
            .collect::<Vec<_>>()
            .len()
    });
    check("repeat_n of a dropped value", || {
        std::iter::repeat_n(Dropped(1), all).collect::<Vec<_>>().len()
    });
    check("own step_by of a counted step", || {
        Up(0).step_by(counted(2)).collect::<Vec<_>>()
    });
    check("own chained take of a counted count", || {
        std::iter::repeat(1u32).chain(Up(0)).take(counted(few)).collect::<Vec<_>>()
    });
    check("repeat_n of a cloned few", || std::iter::repeat_n(Cloned(4), few).collect::<Vec<_>>().len());
    let few_held = Some(few);
    check("repeat_n of a held few", || {
        std::iter::repeat_n(5u32, *few_held.as_ref().unwrap()).collect::<Vec<_>>()
    });
    check("repeat take map of a counted few", || {
        std::iter::repeat(made()).take(counted(few)).map(|x| x * 2).collect::<Vec<_>>()
    });
}
