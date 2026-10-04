//@ run-fail: assertion `left == right` failed\n  left: None\n right: Some(0)
// An `ExactSizeIterator` whose `size_hint` is std's, `(0, None)`, isn't
// exact: std's `len` asserts it is, and panics.
struct Forever;

impl Iterator for Forever {
    type Item = u8;
    fn next(&mut self) -> Option<u8> {
        Some(1)
    }
}

impl ExactSizeIterator for Forever {}

fn main() {
    let it = Forever;
    println!("{}", it.len());
}
