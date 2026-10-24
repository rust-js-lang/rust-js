//@ run-fail: range end index 4 out of range for slice of length 3
// A range that ends one past a slice's length, its bound by the length
// `L + 1`, isn't in bounds: checked, and panics as Rust's does; one that
// ends at the length is JS's own (ADR 0806).
fn all(items: &[u32]) -> Vec<u32> {
    let n = items.len();
    items[..n].to_vec()
}

fn past(items: &[u32]) -> Vec<u32> {
    let n = items.len() + 1;
    items[..n].to_vec()
}

fn main() {
    println!("{:?}", all(&[1, 2, 3]));
    println!("{:?}", past(&[1, 2, 3]));
}
