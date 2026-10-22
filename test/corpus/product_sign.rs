// An integer is never JS's `-0` (ADR 0064): a product its body's ranges show
// in range is JS's own `*` (ADR 0576), but not where it can be `-4 * 0`, and
// `saturating_neg` of 0 is 0. An `f64` made of one shows its sign, and `1.0`
// over it is `inf`, not `-inf`.
use std::hint::black_box;

fn area(width: i32, height: i32) -> i32 {
    width * height
}

fn main() {
    let a: i32 = black_box(-4);
    println!("{:?} {:?}", (a * 0) as f64, 1.0 / ((a * 0) as f64));
    println!("{:?} {:e}", area(-2, 0) as f64, area(-2, 0));
    let b: i8 = black_box(-3);
    println!("{:?}", (b * 0) as f64);
    let z: i32 = black_box(0);
    println!("{:?} {:e}", z.saturating_neg() as f64, z.saturating_neg());
    println!("{:?}", (a * 2) as f64);
}
