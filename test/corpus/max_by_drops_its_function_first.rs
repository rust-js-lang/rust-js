// What `max_by`, `min_by`, `max_by_key` and `min_by_key` drop, and when,
// of the crate's iterator with a destructor: its function, which std's
// `reduce` moves into its fold, before what's left, though it's empty;
// each item passed over as it's passed; and as a panic in the function
// unwinds, the item it was asked of, the one kept, the function, then the
// iterator (#734). Of a key, made once for each, a `Vec`'s items, and an
// empty one, and one whose first step panics, where std's `Map` drops its
// source before the key.
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

struct I(u32);

impl Drop for I {
    fn drop(&mut self) {
        print!("drop i{} ", self.0);
    }
}

struct Items {
    left: u32,
    _held: D,
}

impl Iterator for Items {
    type Item = I;

    fn next(&mut self) -> Option<I> {
        (self.left > 0).then(|| {
            self.left -= 1;
            I(self.left)
        })
    }
}

fn items(left: u32) -> Items {
    Items { left, _held: D("it") }
}

fn closures() {
    let g = D("f");
    let m = it(3).max_by(move |a, b| (a + zero(&g)).cmp(b));
    println!("| max_by {m:?}");
    let g = D("f");
    let m = it(3).min_by(move |a, b| (a + zero(&g)).cmp(b));
    println!("| min_by {m:?}");
}

fn plain() {
    let m = it(3).max_by(|a, b| a.cmp(b));
    println!("| max_by it {m:?}");
    let m = it(3).min_by_key(|a| *a);
    println!("| min_by_key it {m:?}");
    let g = D("f");
    let m = (0..3u32).max_by(move |a, b| (a + zero(&g)).cmp(b));
    println!("| max_by f {m:?}");
    let g = D("f");
    let m = (0..3u32).max_by_key(move |a| a + zero(&g));
    println!("| max_by_key f {m:?}");
}

fn edges() {
    let g = D("f");
    let m = it(0).max_by(move |a, b| (a + zero(&g)).cmp(b));
    println!("| empty {m:?}");
    let m = it(0).min_by_key(|a| *a);
    println!("| empty key {m:?}");
    let g = D("f");
    let m = items(3).max_by(move |a, b| (a.0 + zero(&g)).cmp(&b.0)).map(|i| i.0);
    println!("| items {m:?}");
    let m = items(3).min_by_key(|a| a.0).map(|i| i.0);
    println!("| items key {m:?}");
    let g = D("f");
    let r = catch_unwind(AssertUnwindSafe(|| {
        items(4)
            .max_by(move |a, b| {
                assert!(b.0 != 1, "cmp");
                (a.0 + zero(&g)).cmp(&b.0)
            })
            .map(|i| i.0)
    }));
    println!("| panic {}", r.is_err());
    let r = catch_unwind(AssertUnwindSafe(|| {
        items(4)
            .max_by_key(|a| {
                assert!(a.0 != 1, "key");
                a.0
            })
            .map(|i| i.0)
    }));
    println!("| panic key {}", r.is_err());
    let g = D("f");
    let m = vec![I(5), I(7)]
        .into_iter()
        .max_by_key(move |a| a.0 + zero(&g))
        .map(|i| i.0);
    println!("| vec key {m:?}");
    let g = D("f");
    let m = Vec::<I>::new()
        .into_iter()
        .max_by_key(move |a| a.0 + zero(&g))
        .map(|i| i.0);
    println!("| vec empty key {m:?}");
    let g = D("f");
    let r = catch_unwind(AssertUnwindSafe(|| {
        vec![I(5), I(7)]
            .into_iter()
            .map(|i| {
                assert!(i.0 != 5, "first");
                i
            })
            .max_by_key(move |a| a.0 + zero(&g))
            .map(|i| i.0)
    }));
    println!("| vec first step panics {}", r.is_err());
}

fn main() {
    closures();
    plain();
    edges();
}
