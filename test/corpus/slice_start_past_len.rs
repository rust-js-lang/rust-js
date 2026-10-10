//@ run-fail: range start index 5 out of range for slice of length 4
// `&v[a..b]` with `a` past the end panics with std's `slice_index_fail`: its
// start, checked before its end.
fn main() {
    let v = vec![1, 2, 3, 4];
    let start = v.len() + 1;
    println!("{:?}", &v[start..start + 1]);
}
