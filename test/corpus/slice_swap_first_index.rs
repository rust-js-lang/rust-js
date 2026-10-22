//@ run-fail: index out of bounds: the len is 3 but the index is 4
// A slice's `swap` checks its first index first: of two out of bounds,
// the first's is the one its panic names.
fn main() {
    let mut v = vec![1, 2, 3];
    v.swap(0, 2);
    println!("{v:?}");
    let (a, b) = (v.len() + 1, v.len() + 2);
    v.swap(a, b);
}
