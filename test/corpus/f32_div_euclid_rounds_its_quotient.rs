// An `f32`'s `div_euclid` truncates its quotient as an `f32`, rounded
// first, as std divides `self / rhs` in `f32` (#940).
use std::hint::black_box;

fn main() {
    let a: f32 = black_box(16777218.0);
    println!("{}", a.div_euclid(black_box(-3.5)));
    println!("{}", a.div_euclid(black_box(3.5)));
    println!("{}", (-a).div_euclid(black_box(3.5)));
    println!("{}", a.rem_euclid(black_box(-3.5)));
    let b: f64 = black_box(16777218.0);
    println!("{}", b.div_euclid(black_box(-3.5)));
}
