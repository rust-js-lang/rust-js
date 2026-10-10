//@ run-fail: range start index 5 out of range for slice of length 4
// A `String`'s `drain(a..)` with `a` past the end, checked by `slice::range`
// as a `Vec`'s is.
fn main() {
    let mut s = String::from("abcd");
    let start = s.len() + 1;
    println!("{:?}", s.drain(start..).count());
}
