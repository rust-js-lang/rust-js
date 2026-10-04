//@ run-fail: mid > len
// `split_at` of a slice past its end panics, as Rust's does.

fn main() {
    let v = vec![1, 2];
    let at = v.len() + 1;
    let (before, _) = v.split_at(at);
    println!("{}", before.len());
}
