//@ compile-fail: copying a closure that changes what it captured
// A copy of a `Copy` closure holds its own copy of what it captured by
// value; a JS function shares it, so a copy that could tell is rejected
// (ADR 0246).
fn main() {
    let mut n = 0;
    let mut f = move || {
        n += 1;
        n
    };
    f();
    let mut g = f;
    println!("{} {}", f(), g());
}
