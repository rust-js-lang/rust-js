//@ compile-fail: copying a closure that changes what it captured
// Given where a `Copy` bound lets it be copied, as each call of a `Copy`
// `FnOnce` copies it, a closure that changes what it captured is rejected
// (ADR 0246).
fn twice<F: FnMut() -> i32 + Copy>(mut f: F) -> (i32, i32) {
    let mut g = f;
    (f(), g())
}

fn main() {
    let mut n = 0;
    println!(
        "{:?}",
        twice(move || {
            n += 1;
            n
        })
    );
}
