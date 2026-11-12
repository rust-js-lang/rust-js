// An exported thread-local cell, `export let tickPair`, beside a local of
// the same name in a function that never reads the cell: a temporary named
// by its call, `const tickPair = tickPair$1()` of `tick_pair()`, or a
// parameter. Each reads its own there, so the cell is still its module's
// variable, not refused as one whose box is handed on (#1555).
use std::cell::Cell;

thread_local! {
    pub static TICK_PAIR: Cell<u32> = const { Cell::new(0) };
}

fn tick_pair() -> (u32, u32) {
    let n = TICK_PAIR.get();
    TICK_PAIR.set(n + 1);
    (n, n * 10)
}

fn bump() -> u32 {
    let tick_pair = TICK_PAIR.get() + 5;
    TICK_PAIR.set(tick_pair);
    tick_pair
}

fn show(tick_pair: (u32, u32)) -> String {
    format!("{tick_pair:?}")
}

pub fn ticks() -> u32 {
    TICK_PAIR.get()
}

fn main() {
    println!("{:?}", tick_pair());
    println!("{:?} {}", tick_pair(), ticks());
    println!("{} {}", bump(), ticks());
    println!("{} {}", show(tick_pair()), ticks());
}
