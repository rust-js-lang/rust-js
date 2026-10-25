// Room for a length that's asked, past `isize::MAX` bytes, is a panic as
// std checks it (ADR 0774, amended): `resize` and `resize_with` to a
// length, of what each is given before it's made, its item dropped as it
// unwinds; `collect()` of a range, which knows its length, before it maps
// any; and a `HashMap`'s or `HashSet`'s `with_capacity` and `reserve`, of
// hashbrown's own message. Room that fits is no panic.
use std::any::Any;
use std::collections::{HashMap, HashSet};
use std::panic::{self, AssertUnwindSafe};

struct Noisy(&'static str);

impl Drop for Noisy {
    fn drop(&mut self) {
        println!("dropped {}", self.0);
    }
}

impl Clone for Noisy {
    fn clone(&self) -> Self {
        Noisy(self.0)
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

fn made(i: usize) -> u8 {
    println!("made {i}");
    i as u8
}

// Past half of `usize::MAX`: more than `isize::MAX` of bytes, whatever
// `usize` is.
fn past_half() -> usize {
    usize::MAX / 2 + 1
}

fn main() {
    let mut bytes = vec![1u8, 2, 3];
    check("resize", || bytes.resize(usize::MAX, 0));
    check("resize(5)", || bytes.resize(5, 9));
    check("resize(2)", || bytes.resize(2, 0));
    println!("{bytes:?}");
    let mut noisy = vec![Noisy("kept")];
    check("resize Noisy", || noisy.resize(past_half(), Noisy("big")));
    check("resize Noisy(2)", || noisy.resize(2, Noisy("two")));
    println!("{}", noisy.len());
    let mut made_ones = vec![0u8];
    check("resize_with", || made_ones.resize_with(usize::MAX, || made(0)));
    check("resize_with(3)", || made_ones.resize_with(3, || made(1)));
    println!("{made_ones:?}");

    check("collect", || (0..usize::MAX).map(made).collect::<Vec<u8>>().len());
    check("collect u16", || {
        (0..past_half()).map(|i| i as u16).collect::<Vec<u16>>().len()
    });
    check("collect inclusive", || (0..=usize::MAX).collect::<Vec<usize>>().len());
    check("collect rev", || {
        (0..usize::MAX).rev().map(made).collect::<Vec<u8>>().len()
    });
    let n = std::hint::black_box(3);
    check("collect(3)", || (0..n).map(made).collect::<Vec<u8>>());
    check("collect empty", || (n..1).collect::<Vec<usize>>());

    check("HashMap::with_capacity", || {
        HashMap::<u32, u32>::with_capacity(usize::MAX).len()
    });
    check("HashSet::with_capacity", || {
        HashSet::<u64>::with_capacity(usize::MAX / 8 + 1).len()
    });
    check("HashSet<()>::with_capacity", || {
        HashSet::<()>::with_capacity(usize::MAX).len()
    });
    check("HashMap::with_capacity(10)", || {
        HashMap::<u32, String>::with_capacity(10).len()
    });
    let mut m = HashMap::from([(1u8, "one")]);
    check("HashMap::reserve", || m.reserve(usize::MAX));
    check("HashMap::reserve(4)", || m.reserve(4));
    let mut s = HashSet::from([String::from("a")]);
    check("HashSet::reserve", || s.reserve(past_half()));
    check("HashSet::reserve(n)", || s.reserve(n));
    println!("{} {}", m.len(), s.len());
}
