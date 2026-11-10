//@ run-abort
// A stack overflow in code that drops nothing: no scope catches it, so it
// leaves `main`, which Rust's runtime aborts the process of, exit 134
// (#1193).
fn depth(n: u64) -> u64 {
    if n == u64::MAX { 0 } else { depth(n + 1) + 1 }
}

fn main() {
    println!("{}", depth(0));
}
