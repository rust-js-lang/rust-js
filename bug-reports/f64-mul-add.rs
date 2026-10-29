// f64::mul_add is a fused multiply-add. A finite product that overflows,
// added to an infinity of the opposite sign, is that infinity. rust-js
// returns NaN.
fn main() {
    println!("{}", f64::MAX.mul_add(2.0, f64::NEG_INFINITY));
    println!("{}", (1.0e308_f64).mul_add(-1.0e308, f64::INFINITY));
    println!("{}", f64::MIN.mul_add(2.0, f64::INFINITY));
    // Same-sign overflow and a finite addend stay infinite. Included so the
    // failure is the opposite-infinity case, not every overflowing mul_add.
    println!("{}", f64::MAX.mul_add(2.0, -1.0));
}
