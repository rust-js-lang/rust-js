// `a.mul_add(b, c)` is `a * b + c` rounded once, as a fused multiply-add
// is: `$mulAdd(a, b, c)`, or `$mulAddF32` of `f32`s, which give what
// rounding `a * b` first would lose, a sum past a float's range whose
// product isn't, a subnormal's tie, and each zero's sign.

fn show(x: f64) {
    println!("{:?} {:#x}", x, x.to_bits());
}

fn show32(x: f32) {
    println!("{:?} {:#x}", x, x.to_bits());
}

fn main() {
    let tiny = f64::from_bits(1);
    let close = 1.0 + 2f64.powi(-30);
    let cases: [(f64, f64, f64); 12] = [
        (2.0, 3.0, 4.0),
        (0.1, 10.0, -1.0),
        (close, close, -(1.0 + 2f64.powi(-29))),
        (f64::MAX, 2.0, -f64::MAX),
        (tiny, 0.5, 0.0),
        (tiny, 0.5, tiny),
        (tiny, 1.5, 0.0),
        (-0.0, 1.0, -0.0),
        (1.0, -1.0, 1.0),
        (-1e-300, 1e-300, 0.0),
        (f64::INFINITY, 0.0, 1.0),
        (f64::NAN, 1.0, 1.0),
    ];
    for (a, b, c) in cases {
        show(a.mul_add(b, c));
    }
    show(f64::INFINITY.mul_add(1.0, f64::NEG_INFINITY));
    show(1e308f64.mul_add(10.0, 0.0));
    show(0.0f64.mul_add(-2.0, std::f64::consts::E));

    // An `f32`'s, rounded once to an `f32`: `a * b` is a tie, which `c`
    // tips, though too little for an `f64` to hold.
    let a = 1.0f32 + 2f32.powi(-12);
    show32(a.mul_add(a, 2f32.powi(-60)));
    show32(a.mul_add(a, -2f32.powi(-60)));
    show32(1.0f32.mul_add(2.0, 5.0));
    show32(f32::MAX.mul_add(2.0, -f32::MAX));
    show32(f32::MAX.mul_add(1.5, 0.0));
    show32(f32::from_bits(1).mul_add(0.5, 0.0));
    show32(f32::from_bits(1).mul_add(1.5, 0.0));
    show32(f32::from_bits(1).mul_add(0.5, f32::from_bits(1)));
    show32((-0.0f32).mul_add(0.0, 0.0));
}
