//@ compile-fail: copying a closure that changes what it captured
// A copy of what holds a `Copy` closure that changes what it captured, a
// tuple here, copies the closure, whose own captures a JS function would
// share: one the crate never clones is rejected, as a copy of the closure
// itself is (ADR 0246, #1211).
fn main() {
    let mut n = 0;
    let pair = (
        move || {
            n += 1;
            n
        },
        1,
    );
    let mut a = pair;
    let mut b = pair;
    println!("{} {}", (a.0)(), (b.0)());
}
