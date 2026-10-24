//@ run-fail: range end index 3 out of range for slice of length 1
// A test held in a `bool` says nothing once what it reads is given away by
// `&mut`: `truncate` makes `truncated` a test of the length the `Vec` had,
// so its range is checked, and panics as Rust's does (ADR 0806).
const LIMIT: usize = 3;

fn changed(mut items: Vec<u32>) -> Vec<u32> {
    let truncated = items.len() > LIMIT;
    items.truncate(1);
    if truncated { items[..LIMIT].to_vec() } else { Vec::new() }
}

fn main() {
    println!("{:?}", changed(vec![1, 2]));
    println!("{:?}", changed(vec![1, 2, 3, 4]));
}
