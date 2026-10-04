//@ run-fail: no entry found for key
// `m[&k]` of a key a map hasn't got panics with Rust's message.
use std::collections::HashMap;

fn main() {
    let mut stock = HashMap::new();
    stock.insert("pen", 3);
    println!("{}", stock["pen"]);
    println!("{}", stock["ink"]);
}
