// Room asked past `isize::MAX` bytes is a `capacity overflow` panic, as
// std checks it (ADR 0774): of `with_capacity`, `reserve`, `vec![x; n]`
// and `repeat`, by the items' size, of none by their count alone; an item
// given to `vec!` made first, and dropped as it unwinds. Room that fits is
// no panic.
use std::any::Any;
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

fn made() -> u8 {
    println!("made");
    7
}

fn made_text() -> String {
    println!("made text");
    "ab".to_string()
}

// Past half of `usize::MAX`: more than `isize::MAX` of bytes, whatever
// `usize` is.
fn past_half() -> usize {
    usize::MAX / 2 + 1
}

fn main() {
    check("Vec<u8>::with_capacity", || Vec::<u8>::with_capacity(past_half()).len());
    check("Vec<u16>::with_capacity", || Vec::<u16>::with_capacity(usize::MAX / 4 + 1).len());
    check("Vec<u64>::with_capacity(10)", || Vec::<u64>::with_capacity(10).len());
    check("Vec<()>::with_capacity", || Vec::<()>::with_capacity(usize::MAX).len());
    check("VecDeque::with_capacity", || std::collections::VecDeque::<u32>::with_capacity(past_half()).len());
    check("BinaryHeap::with_capacity", || std::collections::BinaryHeap::<u32>::with_capacity(past_half()).len());
    check("String::with_capacity", || String::with_capacity(usize::MAX).len());

    let mut v = vec![1u8, 2, 3];
    check("reserve", || v.reserve(past_half()));
    check("reserve_exact", || v.reserve_exact(usize::MAX - 1));
    check("reserve(5)", || v.reserve(5));
    check("reserve(0)", || v.reserve(0));
    let mut units = vec![(), (), ()];
    check("Vec<()>::reserve over", || units.reserve(usize::MAX - 2));
    check("Vec<()>::reserve", || units.reserve(usize::MAX - 3));
    let mut s = String::from("ab");
    check("String::reserve", || s.reserve(usize::MAX - 1));
    check("String::reserve(8)", || s.reserve(8));
    s.push('é');
    println!("{s} {}", s.len());

    check("vec![0u8; n]", || vec![0u8; usize::MAX].len());
    check("vec![made(); n]", || vec![made(); past_half()].len());
    check("vec![String; n]", || vec![String::from("x"); past_half()].len());
    check("vec![Noisy; n]", || vec![Noisy("big"); past_half()].len());
    check("vec![Noisy; 2]", || vec![Noisy("two"); 2].len());
    check("vec![(); n]", || vec![(); 0].len());

    check("str::repeat", || "ab".repeat(past_half()).len());
    check("made repeat", || made_text().trim().repeat(past_half()).len());
    let text = String::from("é");
    check("String repeat", || text.repeat(usize::MAX).len());
    check("empty repeat", || "".repeat(usize::MAX).len());
    check("repeat(3)", || "ab".repeat(3));
    check("slice repeat", || [1u8, 2].repeat(past_half()).len());
    check("slice repeat(2)", || [1u16, 2].repeat(2));
}
