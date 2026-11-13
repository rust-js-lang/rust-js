// `max_by_key` and `min_by_key` of the crate's iterator with a `Drop`, or a
// type parameter's: std's `map(|x| (f(&x), x)).reduce(..)` drops the
// iterator as its fold ends, then the kept pair's key as `let (_, x)` takes
// it apart. The kept key dropped first (#1649). And of a key function that
// drops, over a type parameter's, which std drops after the iterator where
// its first step ends; it dropped first. Lent by `by_ref()`, its lender's.
use std::panic::{AssertUnwindSafe, catch_unwind};

#[derive(PartialEq, Eq, PartialOrd, Ord)]
struct K(u32);

impl Drop for K {
    fn drop(&mut self) {
        println!("drop key {}", self.0);
    }
}

struct Loud(&'static str);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

struct Up(u32, u32);

impl Iterator for Up {
    type Item = u32;

    fn next(&mut self) -> Option<u32> {
        (self.0 < self.1).then(|| {
            self.0 += 1;
            self.0
        })
    }
}

impl Drop for Up {
    fn drop(&mut self) {
        println!("drop Up");
    }
}

fn best<I: Iterator<Item = u32>>(it: I) -> Option<u32> {
    it.max_by_key(|&x| K(x))
}

fn least_by<I: Iterator<Item = u32>>(it: I) -> Option<u32> {
    let held = Loud("f");
    it.min_by_key(move |&x| {
        let _ = &held;
        x % 3
    })
}

fn main() {
    println!("{:?}", Up(0, 3).max_by_key(|&x| K(x)));
    println!("{:?}", Up(0, 3).min_by_key(|&x| K(x)));
    println!("{:?}", Up(0, 0).max_by_key(|&x| K(x)));
    println!("{:?}", Up(0, 1).min_by_key(|&x| K(x)));
    println!("{:?}", Up(0, 3).map(|x| x * 2).max_by_key(|&x| K(x)));
    println!("{:?}", best(Up(0, 3)));
    println!("{:?}", best(Up(0, 0)));
    println!("{:?}", least_by(Up(0, 3)));
    println!("{:?}", least_by(Up(0, 0)));
    println!("{:?}", best(vec![1, 5, 2].into_iter()));
    println!("{:?}", least_by([4, 2].into_iter()));

    let mut up = Up(0, 3);
    println!("{:?}", up.by_ref().max_by_key(|&x| K(x)));
    drop(up);

    let r = catch_unwind(AssertUnwindSafe(|| {
        Up(0, 3).max_by_key(|&x| {
            assert!(x != 3, "key");
            K(x)
        })
    }));
    println!("{}", r.is_err());
}
