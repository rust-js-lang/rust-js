//! A consumer of a library's generic `clone_from_slice`, given numbers,
//! a `#[derive(Clone, Copy)]` struct, a hand-written `Clone` of a `Copy`
//! type and `String`s, directly, built into a tuple, relayed by the
//! library and by its own generic code: each panics with native Rust's
//! message (#2318).
use dep::{refill, refill_tagged, relay};
use std::any::Any;
use std::panic::{self, AssertUnwindSafe};

#[derive(Clone, Copy, Debug)]
struct Point(i32, i32);

#[derive(Copy, Debug)]
struct Noisy(i32);

impl Clone for Noisy {
    #[allow(clippy::non_canonical_clone_impl)]
    fn clone(&self) -> Self {
        println!("clone {}", self.0);
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

fn check(what: &str, f: impl FnOnce()) {
    match panic::catch_unwind(AssertUnwindSafe(f)) {
        Ok(()) => println!("{what}: cloned"),
        Err(payload) => println!("{what}: {}", message(payload)),
    }
}

fn own<V: Clone>(into: &mut [V], from: &[V]) {
    relay(into, from);
}

pub fn main() {
    let mut numbers = [1u16, 2];
    check("numbers", || refill(&mut numbers, &[3]));
    let mut points = [Point(0, 0)];
    check("points", || refill(&mut points, &[]));
    let mut noisy = [Noisy(0)];
    check("noisy", || refill(&mut noisy, &[Noisy(1), Noisy(2)]));
    let mut words = [String::new()];
    check("words", || refill(&mut words, &[]));
    let mut tagged = [(1i64, 0u8)];
    check("tagged", || refill_tagged(&mut tagged, &[]));
    let mut tagged_words = [(String::new(), 0u8)];
    check("tagged words", || refill_tagged(&mut tagged_words, &[]));
    let mut relayed = ['a', 'b'];
    check("relayed", || relay(&mut relayed, &['c']));
    let mut owned = [1.5f64];
    check("own", || own(&mut owned, &[]));
    let mut owned_words = [String::new()];
    check("own words", || own(&mut owned_words, &[]));
    let mut same = [Point(0, 0)];
    check("same", || refill(&mut same, &[Point(3, 4)]));
    println!("{same:?}");
}
