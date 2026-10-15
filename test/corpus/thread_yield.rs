// `thread::yield_now()` of the one thread a program has (ADR 0142) yields
// to none: nothing, in JS (ADR 0315).
use std::thread;

fn main() {
    let mut i = 0;
    while i < 3 {
        i += 1;
        println!("{i}");
        thread::yield_now();
    }
}
