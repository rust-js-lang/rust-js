//@ run-fail: range end index 3 out of range for slice of length 2
// A test held in a `bool` says nothing once what it reads has changed: the
// `pop` makes `truncated` a test of the length the `Vec` had, so its range
// is checked, and panics as Rust's does (ADR 0806).
const LIMIT: usize = 3;

fn popped(mut items: Vec<u32>) -> Vec<u32> {
    let truncated = items.len() >= LIMIT;
    items.pop();
    if truncated { items[..LIMIT].to_vec() } else { Vec::new() }
}

fn main() {
    println!("{:?}", popped(vec![1, 2, 3, 4]));
    println!("{:?}", popped(vec![1, 2, 3]));
}
