// `powf` of a constant base but ±1 is JS's `**`, `2 ** n`: the same value of
// every power, NaN and the infinities too. Of 1 or -1, whose powers JS's `**`
// makes NaN where Rust's are 1, it's as before.

fn doubled(n: f64) -> f64 {
    2f64.powf(n)
}

fn tenth(n: f64) -> f64 {
    0.1f64.powf(n)
}

fn of_one(n: f64) -> f64 {
    1f64.powf(n)
}

fn of_minus_one(n: f64) -> f64 {
    (-1f64).powf(n)
}

fn main() {
    for n in [
        0.0,
        -0.0,
        3.0,
        -2.5,
        0.5,
        1e10,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NAN,
    ] {
        println!("{:?} {:?} {:?} {:?}", doubled(n), tenth(n), of_one(n), of_minus_one(n));
    }
}
