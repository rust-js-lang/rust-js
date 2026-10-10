//@ run-fail: range end index 6 out of range for slice of length 4
// `extend_from_within(a..b)` is checked by `slice::range`, its end first, not
// as `&v[a..b]` is.
fn main() {
    let mut v = vec![1, 2, 3, 4];
    let start = v.len() + 1;
    v.extend_from_within(start..start + 1);
    println!("{v:?}");
}
