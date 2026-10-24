//@ run-fail: range end index 3 out of range for slice of length 1
// A `bool` that holds a test on one path and is set on the other holds
// none where they meet: its range is checked, and panics as Rust's does
// (ADR 0806).
fn forced(items: &[u32], force: bool) -> Vec<u32> {
    let mut truncated = items.len() > 3;
    if !force {
        println!("as tested");
    } else {
        truncated = true;
    }
    if truncated { items[..3].to_vec() } else { items.to_vec() }
}

fn main() {
    println!("{:?}", forced(&[1, 2, 3, 4], false));
    println!("{:?}", forced(&[1], true));
}
