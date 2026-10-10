//@ run-fail: range end index 9 out of range for slice of length 4
// `&mut v[a..b]` is checked as it's made, as `&v[a..b]` is.
fn main() {
    let mut v = vec![1, 2, 3, 4];
    let end = v.len() + 5;
    let part = &mut v[2..end];
    println!("made");
    part[0] = 0;
}
