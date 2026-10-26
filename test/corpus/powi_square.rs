// A variable's square, `x.powi(2)`, is `x * x`, as Rust's multiplies once:
// the same value of each `f64`, NaN, -0 and the infinities too. Another
// power, a square of what isn't a variable, and an `f32`'s are as before.

fn square(x: f64) -> f64 {
    x.powi(2)
}

fn distance(a: (f64, f64), b: (f64, f64)) -> f64 {
    let dx = a.0 - b.0;
    let dy = a.1 - b.1;
    (dx.powi(2) + dy.powi(2)).sqrt()
}

fn cube(x: f64) -> f64 {
    x.powi(3)
}

fn square_of_sum(a: f64, b: f64) -> f64 {
    (a + b).powi(2)
}

fn narrow_square(x: f32) -> f32 {
    x.powi(2)
}

fn squares(xs: &[f64]) -> Vec<f64> {
    xs.iter().map(|x| x.powi(2)).collect()
}

fn main() {
    for x in [
        0.0,
        -0.0,
        1.5,
        -3.0,
        1e200,
        f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        0.1,
    ] {
        println!("{:?} {:?}", square(x), 1.0 / square(x));
    }
    println!("{}", distance((0.0, 0.0), (3.0, 4.0)));
    println!("{} {}", cube(-2.0), square_of_sum(0.1, 0.2));
    println!("{} {}", narrow_square(0.1), narrow_square(3.0e30));
    println!("{:?}", squares(&[0.5, -2.0]));
}
