// Number methods apps use: clamping, parsing in a radix, dividing up,
// a float's sign and fraction, angles, and integer logarithms and roots.

fn main() {
    // Clamped, into a range, an integer's or a float's.
    println!(
        "{} {} {} {} {}",
        17i32.clamp(0, 10),
        (-3i64).clamp(-2, 2),
        200u8.clamp(1, 100),
        1.5f64.clamp(0.0, 1.0),
        f64::NAN.clamp(0.0, 1.0)
    );
    let level = 7u32;
    println!("{}", level.clamp(1, 5) * 2);

    // Parsed in a radix, and errors where Rust finds them.
    println!(
        "{:?} {:?} {:?} {:?}",
        i32::from_str_radix("-ff", 16),
        u8::from_str_radix("11111111", 2),
        u64::from_str_radix("777", 8),
        i64::from_str_radix("Zz", 36)
    );
    for s in ["", "+", "-1", "1g", "100000000", "-80000000", "+7f"] {
        println!(
            "{s:?}: {:?} {:?}",
            u32::from_str_radix(s, 16).map_err(|e| e.to_string()),
            i32::from_str_radix(s, 16).map_err(|e| e.to_string())
        );
    }

    // Divided, rounding up.
    println!("{} {} {} {}", 7u32.div_ceil(2), 8u32.div_ceil(2), 0u8.div_ceil(3), 10u64.div_ceil(3));

    // A float's sign, fraction and angle.
    for x in [-2.5f64, 0.0, -0.0, 3.75] {
        println!(
            "{} {} {} {} {}",
            x.signum(),
            x.fract(),
            x.is_sign_negative(),
            x.is_sign_positive(),
            x.to_radians()
        );
    }
    println!("{} {}", f64::NAN.signum().is_nan(), std::f64::consts::PI.to_degrees());
    println!("{} {} {}", 45f32.to_radians(), 1f32.to_degrees(), (-0.5f32).fract());

    // Integer logarithms, roots, midpoints and zeros.
    println!("{} {} {} {}", 1000u32.ilog10(), 999u32.ilog10(), 8u32.ilog2(), 255u8.ilog2());
    println!("{} {} {}", 1u64.ilog10(), u64::MAX.ilog2(), 12345i64.ilog10());
    println!("{} {} {} {}", 17u32.isqrt(), 0u8.isqrt(), 99i32.isqrt(), 10u64.pow(18).isqrt());
    println!(
        "{} {} {} {} {}",
        3u32.midpoint(8),
        (-7i32).midpoint(0),
        0i32.midpoint(-7),
        u8::MAX.midpoint(253),
        i64::MIN.midpoint(-1)
    );
    println!("{} {} {}", 5u8.count_zeros(), (-1i32).count_zeros(), 0u64.count_zeros());
}
