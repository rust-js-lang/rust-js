//! `made`, whose JS gives one function where Rust makes two `Rc`s, which a
//! consumer that compares them compiles itself, reads a static private to
//! its library, which only the library's code can reach (#1052).
use std::rc::Rc;
use std::sync::atomic::{AtomicU32, Ordering};

static MADE: AtomicU32 = AtomicU32::new(0);

pub fn made() -> (Rc<dyn Fn() -> u32>, Rc<dyn Fn() -> u32>) {
    let n = MADE.fetch_add(1, Ordering::Relaxed);
    let read = move || n;
    (Rc::new(read), Rc::new(read))
}
