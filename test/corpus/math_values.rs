// A std function or constant that's JS's `Math`'s, as a value (ADR 0887):
// `u32::leading_zeros` taken as a `fn` is `Math.clz32`, `f64::ln` is
// `Math.log`, `std::f64::consts::LN_2` is `Math.LN2`. Each is called and
// read here against native Rust; one that isn't `Math`'s for every value,
// `f64::round` or an `f32`'s `sqrt`, stays the arrow of its call. What a
// math library computes is shown to 12 places: its last bit is the
// platform's, natively.

use std::f64::consts;

// React's `clz32` polyfill: the fallback by the natural logarithm, which
// `Math.clz32` is chosen over where the host has it.
const LOG: fn(f64) -> f64 = f64::ln;
const LN2: f64 = consts::LN_2;
// Written as a literal, or computed, it's the number.
const HALF_LN2: f64 = consts::LN_2 / 2.0;
const WRITTEN: f64 = 0.6931471805599453;

fn clz32_fallback(x: u32) -> u32 {
    let as_uint = x;
    if as_uint == 0 {
        return 32;
    }
    31 - (LOG(as_uint as f64) / LN2) as u32
}

fn clz32(native: bool) -> fn(u32) -> u32 {
    if native {
        u32::leading_zeros
    } else {
        clz32_fallback
    }
}

fn exact(name: &str, f: fn(f64) -> f64) {
    let shown: Vec<String> = [-2.5, -0.5, -0.0, 0.0, 0.5, 1.5, 7.0, f64::INFINITY, f64::NAN]
        .iter()
        .map(|&x| format!("{:?}", f(x)))
        .collect();
    println!("{name}: {}", shown.join(" "));
}

fn approximate(name: &str, f: fn(f64) -> f64, inputs: &[f64]) {
    let shown: Vec<String> = inputs.iter().map(|&x| format!("{:.12}", f(x))).collect();
    println!("{name}: {}", shown.join(" "));
}

// Each as an `f64`, so one that isn't an `f32`, a double's `sqrt`, shows.
fn single(name: &str, f: fn(f32) -> f32) {
    let shown: Vec<String> = [-2.5f32, -0.0, 0.1, 1.5, 2.0, f32::NAN]
        .iter()
        .map(|&x| format!("{:?}", f64::from(f(x))))
        .collect();
    println!("{name}: {}", shown.join(" "));
}

fn main() {
    // The polyfill, both ways, and `Math.clz32` of every kind of `u32`.
    for x in [0, 1, 3, 7, 100, 1000, 12_345_678, 0x7fff_ffff, 0x8000_0000, u32::MAX] {
        println!("clz32({x}) = {} {}", clz32(true)(x), clz32(false)(x));
    }
    let signed: fn(i32) -> u32 = i32::leading_zeros;
    let size: fn(usize) -> u32 = usize::leading_zeros;
    let narrow: fn(u8) -> u32 = u8::leading_zeros;
    let short: fn(i16) -> u32 = i16::leading_zeros;
    println!(
        "{} {} {} {} {} {} {}",
        signed(-1),
        signed(i32::MIN),
        signed(1),
        size(usize::MAX),
        narrow(1),
        short(-1),
        short(1)
    );
    let unsigned_abs: fn(i32) -> u32 = i32::unsigned_abs;
    let narrow_abs: fn(i8) -> u8 = i8::unsigned_abs;
    let signum: fn(i32) -> i32 = i32::signum;
    let small_signum: fn(i8) -> i8 = i8::signum;
    println!(
        "{} {} {} {} {} {} {}",
        unsigned_abs(i32::MIN),
        unsigned_abs(-5),
        narrow_abs(i8::MIN),
        signum(i32::MIN),
        signum(0),
        signum(9),
        small_signum(-3)
    );
    // Called, as `Math`'s are.
    let (n, m) = (-7i32, i32::MIN);
    println!("{} {} {} {}", n.unsigned_abs(), m.unsigned_abs(), n.signum(), n.leading_zeros());

    // An `f64`'s exact functions, `-0.0` and `NaN` among what they're given.
    exact("floor", f64::floor);
    exact("ceil", f64::ceil);
    exact("trunc", f64::trunc);
    exact("abs", f64::abs);
    exact("sqrt", f64::sqrt);
    // Not `Math.round`, which takes a half up: Rust's, away from zero.
    exact("round", f64::round);
    // Not `Math.sign`, `0` of a `0.0`: Rust's `1.0`.
    exact("signum", f64::signum);

    let wide = [0.1, 0.5, 1.0, 2.5, 3.0];
    approximate("cbrt", f64::cbrt, &wide);
    approximate("exp", f64::exp, &wide);
    approximate("ln", LOG, &wide);
    approximate("log10", f64::log10, &wide);
    approximate("log2", f64::log2, &wide);
    approximate("exp_m1", f64::exp_m1, &wide);
    approximate("ln_1p", f64::ln_1p, &wide);
    approximate("sin", f64::sin, &wide);
    approximate("cos", f64::cos, &wide);
    approximate("tan", f64::tan, &wide);
    approximate("sinh", f64::sinh, &wide);
    approximate("cosh", f64::cosh, &wide);
    approximate("tanh", f64::tanh, &wide);
    approximate("asinh", f64::asinh, &wide);
    approximate("atan", f64::atan, &wide);
    let unit = [-1.0, -0.5, 0.0, 0.5, 1.0];
    approximate("asin", f64::asin, &unit);
    approximate("acos", f64::acos, &unit);
    approximate("atanh", f64::atanh, &[-0.5, 0.0, 0.25, 0.5]);
    approximate("acosh", f64::acosh, &[1.0, 1.5, 10.0]);

    // Of two numbers, given in order; handed to an iterator, given no more
    // than Rust gives it.
    let atan2: fn(f64, f64) -> f64 = f64::atan2;
    let hypot: fn(f64, f64) -> f64 = f64::hypot;
    println!("{:.12} {:.12} {:?}", atan2(1.0, -1.0), atan2(-0.0, 1.0), hypot(3.0, 4.0));
    println!("{:?}", [3.0, 4.0, 12.0].into_iter().fold(0.0, hypot));
    let sqrt: fn(f64) -> f64 = f64::sqrt;
    let roots: Vec<f64> = [4.0, 9.0, 2.0].into_iter().map(sqrt).collect();
    println!("{roots:?}");

    // An `f32`'s exact ones, and its `sqrt`, rounded to an `f32`.
    single("f32 floor", f32::floor);
    single("f32 ceil", f32::ceil);
    single("f32 trunc", f32::trunc);
    single("f32 abs", f32::abs);
    single("f32 sqrt", f32::sqrt);

    // `Math`'s constants, to their bits, and those it hasn't.
    for (name, value) in [
        ("E", consts::E),
        ("LN_2", consts::LN_2),
        ("LN_10", consts::LN_10),
        ("LOG2_E", consts::LOG2_E),
        ("LOG10_E", consts::LOG10_E),
        ("PI", consts::PI),
        ("SQRT_2", consts::SQRT_2),
        ("FRAC_1_SQRT_2", consts::FRAC_1_SQRT_2),
        ("TAU", consts::TAU),
        ("LN2", LN2),
        ("HALF_LN2", HALF_LN2),
        ("WRITTEN", WRITTEN),
    ] {
        println!("{name} = {value:?} {:#x}", value.to_bits());
    }
    println!("{:?} {:?}", std::f32::consts::PI, 2.0 * consts::PI);
}
