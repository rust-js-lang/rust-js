//@ compile-fail: copying a closure that changes what it captured
// A copy of a `Copy` struct holding a closure that changes what it
// captured copies the closure, whose own captures a JS function would
// share: one the crate never clones is rejected (ADR 0246, #1211).
#[derive(Clone, Copy)]
struct Ticker<F> {
    tick: F,
}

fn main() {
    let mut n = 0;
    let t = Ticker {
        tick: move || {
            n += 1;
            n
        },
    };
    let mut a = t;
    let mut b = t;
    println!("{} {}", (a.tick)(), (b.tick)());
}
