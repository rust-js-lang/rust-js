//@ run-fail: range start index 5 out of range for slice of length 4
// `drain(a..)` with `a` past the end, checked by std's `slice::range`: its
// start is out of range.
fn main() {
    let mut v = vec![1, 2, 3, 4];
    let start = v.len() + 1;
    println!("{:?}", v.drain(start..).count());
}
