// An `enumerate()` of a slice's method that yields more than its items,
// `split`, whose pieces are one more than its separators, bounds nothing of
// its index: `v[i]` past the `Vec`'s own items panics as Rust's does, never
// `undefined` (#363). The index is a call's, `Vec`'s `Index`, which THIR's
// ranges decide until #1672's switch.
use std::panic::{self, AssertUnwindSafe};

fn summed(v: Vec<u32>) -> u32 {
    let mut total = 0;
    for (i, _) in v.split(|x| *x == 0).enumerate() {
        total += v[i];
    }
    total
}

fn summed_of_a_slice(v: Vec<u32>) -> u32 {
    let s: &[u32] = &v;
    let mut total = 0;
    for (i, _) in s.split(|x| *x == 0).enumerate() {
        total += v[i];
    }
    total
}

fn main() {
    println!("{}", summed(vec![1, 2, 3]));
    let r = panic::catch_unwind(AssertUnwindSafe(|| summed(vec![0, 0])));
    println!("{}", r.is_err());
    let r = panic::catch_unwind(AssertUnwindSafe(|| summed_of_a_slice(vec![0, 0])));
    println!("{}", r.is_err());
}
