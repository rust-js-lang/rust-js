// A `rev()` of a range's `rev()` is the range itself, stepped from its
// start, as std's `Rev` of a `Rev` steps what it holds: no `$rev` of a
// `$rev` (#205).
fn main() {
    let doubled: Vec<u32> = (0..4u32).rev().rev().map(|x| x * 2).collect();
    println!("{doubled:?}");
    let total: i32 = (1..=3).rev().rev().map(|x| x * 10).sum();
    println!("{total}");
}
