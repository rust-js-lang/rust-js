//! A library's thread-locals: a flag, which can't change, as React's
//! `shared` has `enableProfilerTimer`, and a count it sets.
use std::cell::Cell;

thread_local! {
    pub static ENABLE_PROFILER_TIMER: bool = true;
    pub static COUNT: Cell<u32> = const { Cell::new(0) };
}

pub fn bump() {
    COUNT.set(COUNT.get() + 1);
}
