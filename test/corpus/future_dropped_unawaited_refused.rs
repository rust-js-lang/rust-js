//@ compile-fail: it's dropped unawaited
// A future of the crate's dropped unawaited: Rust's never runs its body; a
// JS promise runs as it's made, so its print would show. rust-js refuses it
// (#965, ADR 0029 amended); one whose body prints nothing, nor panics, is
// as Rust's either way and compiles.
async fn greet() {
    println!("hello");
}

pub async fn never() {
    let f = greet();
    drop(f);
}

fn main() {
    let _ = never;
}
