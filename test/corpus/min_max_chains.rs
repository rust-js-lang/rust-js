// A chain of an integer's `min`s, or of its `max`s, is one call of
// `Math`'s, `Math.min(a, b, c)`, as JS writes it: the same order, the same
// number. Of both, each its own; of floats, their helpers, as NaN tells.

fn least(a: u32, b: u32, c: u32) -> u32 {
    a.min(b).min(c)
}

fn most(a: i32, b: i32, c: i32) -> i32 {
    a.max(b.max(c))
}

fn clamped(a: i64, low: i64, high: i64) -> i64 {
    a.max(low).min(high)
}

fn least_float(a: f64, b: f64, c: f64) -> f64 {
    a.min(b).min(c)
}

fn main() {
    println!("{} {} {}", least(3, 1, 2), least(0, u32::MAX, 7), least(9, 9, 9));
    println!("{} {}", most(-3, -1, -2), most(i32::MIN, 0, i32::MAX));
    println!("{} {} {}", clamped(5, 0, 3), clamped(-5, 0, 3), clamped(2, 0, 3));
    println!(
        "{} {} {}",
        least_float(f64::NAN, 1.0, 2.0),
        least_float(-0.0, 0.0, 1.0),
        least_float(3.0, 2.0, 1.0)
    );
}
