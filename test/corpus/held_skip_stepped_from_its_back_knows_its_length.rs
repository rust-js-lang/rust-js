// A `skip(1)` a variable holds, stepped from its back by a `rev()`, whose
// length a `len()` asks first: its maker made it knowing its back, and
// marked it knowing its length, which the variable keeps, as std's `Skip`
// of an exact-sized source is exact-sized too (#1317).
fn main() {
    let items = vec![1u32, 2, 3, 4, 5];
    let s = items.into_iter().skip(1);
    println!("{}", s.len());
    let r: Vec<u32> = s.rev().collect();
    println!("{r:?}");
}
