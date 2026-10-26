// ADR 0922: what a module exports is named first, by its own name, and an
// item the module keeps to itself is named apart from it: the exported
// thread-local is `tickCount`, as JS's importer reads it, and the private
// function that reads it `tickCount$1`.
use std::cell::Cell;

thread_local! {
    pub static TICK_COUNT: Cell<u32> = const { Cell::new(0) };
}

fn tick_count() -> u32 {
    TICK_COUNT.get()
}

pub fn tick() -> u32 {
    TICK_COUNT.set(TICK_COUNT.get() + 1);
    tick_count() * 10
}

fn main() {
    let first = tick();
    let second = tick();
    println!("{first} {second} {}", tick_count());
}
