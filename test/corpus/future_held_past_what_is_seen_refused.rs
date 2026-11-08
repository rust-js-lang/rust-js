//@ compile-fail: code that may be seen runs before it's awaited
// A future of the crate's held while something that prints runs, then
// awaited: Rust's runs its body at the `.await`, after the print; a JS
// promise runs as it's made, before it. rust-js refuses it (#965, ADR 0029
// amended); awaited where it's made, or through `js::promise`, it compiles.
async fn greet() {
    println!("hello");
}

pub async fn later() {
    let f = greet();
    println!("first");
    f.await;
}

fn main() {
    let _ = later;
}
