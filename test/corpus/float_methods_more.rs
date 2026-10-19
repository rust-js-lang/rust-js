// More of a float's methods, as num-traits calls them: the inverse
// hyperbolics, a logarithm of any base, `sin_cos`, Euclid's remainder and
// quotient, `copysign`, and the deprecated `abs_sub`. What a math library
// computes is shown to 12 places: its last bit is the platform's, natively.

#[allow(deprecated)]
fn main() {
    let x = 0.25f64;
    println!("{:.12} {:.12} {:.12}", (-0.5f64).asinh(), 1.5f64.acosh(), x.atanh());
    println!("{:?} {:?}", 0.5f64.acosh(), 8f64.log(2.0));
    let (s, c) = 1f64.sin_cos();
    println!("{s:.12} {c:.12}");
    println!(
        "{:?} {:?} {:?} {:?}",
        (-7.5f64).rem_euclid(2.0),
        (-7.5f64).div_euclid(2.0),
        7.5f64.rem_euclid(-2.0),
        7f64.div_euclid(-2.0)
    );
    println!(
        "{:?} {:?} {:?}",
        3f64.copysign(-0.0),
        (-3f64).copysign(1.0),
        3f64.copysign(f64::NEG_INFINITY)
    );
    println!(
        "{:?} {:?} {:?}",
        5f64.abs_sub(2.0),
        2f64.abs_sub(5.0),
        f64::NAN.abs_sub(1.0)
    );
    let y = 0.3f32;
    let (sy, cy) = y.sin_cos();
    println!(
        "{:.6} {:.6} {:.6} {sy:.6} {cy:.6}",
        y.asinh(),
        y.atanh(),
        10f32.log(3.0)
    );
    println!(
        "{:?} {:?} {:?}",
        (-7.5f32).rem_euclid(2.0),
        (-7.5f32).div_euclid(2.0),
        3f32.copysign(-0.0)
    );
    // `midpoint`: halfway, an `f64`'s without going past its largest, a
    // tiny one kept whole; an `f32`'s, `f64`s' sum halved.
    println!(
        "{} {:e} {:e} {:e} {} {}",
        1.0f64.midpoint(3.0),
        f64::MAX.midpoint(f64::MAX),
        5e-324f64.midpoint(f64::MAX),
        f64::MAX.midpoint(-5e-324),
        (-1.5f32).midpoint(2.5),
        1.0f32.midpoint(1.0000001) == 1.0
    );
}
