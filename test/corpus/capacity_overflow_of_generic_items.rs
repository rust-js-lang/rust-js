// Room std checks for items whose size a type parameter gives, by their
// bytes against `isize::MAX` (ADR 0774, amended): of `with_capacity`,
// `reserve`, `reserve_exact`, `vec![x; n]`, `resize`, `resize_with` and a
// slice's `repeat`, each in generic code, the size its caller gives,
// `TSize`, so a count past it panics `capacity overflow` as native Rust
// does, on wasm32 too, and one that fits doesn't (#1432). A collect's is
// #1436's, a hash table's #1437's.
use std::any::Any;
use std::panic::{self, AssertUnwindSafe};

fn message(payload: Box<dyn Any + Send>) -> String {
    match payload.downcast::<&str>() {
        Ok(s) => s.to_string(),
        Err(payload) => match payload.downcast::<String>() {
            Ok(s) => *s,
            Err(_) => "?".to_string(),
        },
    }
}

fn check<R>(what: &str, f: impl FnOnce() -> R) {
    match panic::catch_unwind(AssertUnwindSafe(f)) {
        Ok(_) => println!("{what}: ok"),
        Err(payload) => println!("{what}: {}", message(payload)),
    }
}

fn with_capacity<T>(n: usize) -> usize {
    Vec::<T>::with_capacity(n).len()
}

fn reserve<T>(x: T, n: usize) -> usize {
    let mut v = vec![x];
    v.reserve(n);
    v.len()
}

fn reserve_exact<T>(x: T, n: usize) -> usize {
    let mut v = vec![x];
    v.reserve_exact(n);
    v.len()
}

fn repeated<T: Clone>(x: T, n: usize) -> usize {
    vec![x; n].len()
}

fn resized<T: Clone>(x: T, n: usize) -> usize {
    let mut v = vec![x.clone()];
    v.resize(n, x);
    v.len()
}

fn resized_with<T: Clone>(x: T, n: usize) -> usize {
    let mut v = vec![x.clone()];
    v.resize_with(n, || x.clone());
    v.len()
}

fn slice_repeated<T: Copy>(x: T, n: usize) -> usize {
    [x].repeat(n).len()
}

fn main() {
    let past = usize::MAX / 2;
    let wide = [0u64; 4];
    check("with_capacity", || with_capacity::<[u64; 4]>(past));
    check("with_capacity fits", || with_capacity::<[u64; 4]>(3));
    check("reserve", || reserve(wide, past));
    check("reserve fits", || reserve(wide, 3));
    check("reserve_exact", || reserve_exact(wide, past));
    check("vec![x; n]", || repeated(wide, past));
    check("vec![x; n] fits", || repeated(wide, 3));
    check("resize", || resized(wide, past));
    check("resize_with", || resized_with(wide, past));
    check("repeat", || slice_repeated(wide, past));
    check("repeat fits", || slice_repeated(wide, 3));
    check("units", || with_capacity::<()>(past));
}
