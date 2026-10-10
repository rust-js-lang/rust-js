//@ run-fail: mid > len
// `split_at_mut(mid)` past the end panics with std's message.
fn main() {
    let mut v = vec![1, 2, 3, 4];
    let mid = v.len() + 1;
    let (a, _) = v.split_at_mut(mid);
    a[0] = 0;
}
