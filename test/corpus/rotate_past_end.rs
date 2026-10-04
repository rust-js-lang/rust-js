//@ run-fail: assertion failed: mid <= self.len()
// Rotating a slice by more than it has fails std's assertion, which names
// the count `mid`.
fn main() {
    let mut v = vec![1, 2, 3];
    v.rotate_left(2);
    println!("{:?}", v);
    v.rotate_left(4);
}
