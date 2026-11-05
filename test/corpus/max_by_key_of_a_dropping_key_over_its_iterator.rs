// `max_by_key` and `min_by_key` of the crate's iterator with a destructor,
// of a key that owns a value with one: std's `self.map(key).reduce(..)`,
// whose fold drops the key, then what's left of the iterator, but where
// `reduce`'s first step gives nothing, or its key panics, the `Map` drops
// whole, the iterator, then the key; and where a later key panics, the
// fold's order (#789).
use std::panic::{AssertUnwindSafe, catch_unwind};

struct D(&'static str);

impl Drop for D {
    fn drop(&mut self) {
        print!("drop {} ", self.0);
    }
}

struct It {
    left: u32,
    _held: D,
}

impl Iterator for It {
    type Item = u32;

    fn next(&mut self) -> Option<u32> {
        (self.left > 0).then(|| {
            self.left -= 1;
            self.left
        })
    }
}

fn it(left: u32) -> It {
    It { left, _held: D("it") }
}

fn zero(_guard: &D) -> u32 {
    0
}

fn main() {
    let k = D("key");
    let m = it(3).max_by_key(move |x| x + zero(&k));
    println!("| max_by_key {m:?}");
    let k = D("key");
    let m = it(0).max_by_key(move |x| x + zero(&k));
    println!("| empty {m:?}");
    let k = D("key");
    let m = it(3).min_by_key(move |x| x + zero(&k));
    println!("| min_by_key {m:?}");
    let k = D("key");
    let m = it(0).min_by_key(move |x| x + zero(&k));
    println!("| min empty {m:?}");
    let k = D("key");
    let r = catch_unwind(AssertUnwindSafe(|| {
        it(4).max_by_key(move |x| {
            assert!(*x != 1, "key");
            x + zero(&k)
        })
    }));
    println!("| key panics {}", r.is_err());
    let k = D("key");
    let r = catch_unwind(AssertUnwindSafe(|| {
        it(2).max_by_key(move |x| {
            assert!(*x != 1, "first key");
            x + zero(&k)
        })
    }));
    println!("| first key panics {}", r.is_err());
}
