//@ run-fail: range start index 5 out of range for slice of length 3
// A range from past a slice's end, which nothing before it tests, panics
// as Rust's does; one from at most its length is JS's own (ADR 0806).
fn rest(items: &[u32], from: usize) -> Vec<u32> {
    items[from..].to_vec()
}

fn rest_tested(items: &[u32], from: usize) -> Vec<u32> {
    if from <= items.len() { items[from..].to_vec() } else { Vec::new() }
}

fn main() {
    println!("{:?} {:?}", rest_tested(&[1, 2, 3], 3), rest_tested(&[1, 2, 3], 5));
    println!("{:?}", rest(&[1, 2, 3], 1));
    println!("{:?}", rest(&[1, 2, 3], 5));
}
