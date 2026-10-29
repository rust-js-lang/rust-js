// A `Wrapping`'s `op=` of a call's operand, `b0 += b1 << 4`, of a variable a
// copy was made of: the variable rebound, its copies what they were, as
// a `Copy` value's are (#396).
use std::num::Wrapping as W;
fn f(mut b0: W<u32>, b1: W<u32>) -> (u32, u32) {
    let w = b0;
    b0 += b1 << 4;
    (w.0, b0.0)
}
fn main() {
    let a = W(1u32);
    let r = f(a, W(1));
    println!("{} {:?}", a.0, r);
    let mut c = W(5u32);
    let d = c;
    c += d << 1;
    println!("{} {}", c.0, d.0);
}
