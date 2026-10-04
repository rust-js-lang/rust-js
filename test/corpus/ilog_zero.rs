//@ run-fail: argument of integer logarithm must be positive
// The logarithm of zero has none.
fn main() {
    for n in [100u32, 9, 0] {
        println!("{}", n.ilog10());
    }
}
