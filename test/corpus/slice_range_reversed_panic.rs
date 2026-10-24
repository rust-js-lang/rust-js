//@ run-fail: slice index starts at 3 but ends at 1
// A range in a slice long enough, but whose start is past its end, panics
// as Rust's does: in bounds only where `start ≤ end ≤` the length (ADR
// 0806).
fn reversed(items: &[u32], start: usize) -> Vec<u32> {
    if items.len() > 4 && start <= 3 { items[start..1].to_vec() } else { Vec::new() }
}

fn main() {
    println!("{:?}", reversed(&[1, 2, 3, 4, 5], 0));
    println!("{:?}", reversed(&[1, 2, 3, 4, 5], 3));
}
