//@ run-fail: range start index 5 out of range for slice of length 4
// `&v[a..]` with `a` past the end: its start is out of range, not after its end.
fn main() {
    let v = vec![1, 2, 3, 4];
    let start = v.len() + 1;
    println!("{:?}", &v[start..]);
}
