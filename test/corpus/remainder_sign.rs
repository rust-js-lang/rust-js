// An integer is never JS's `-0` (ADR 0064): `-2 % 2` is `0`, which an `f64`
// made of it shows, and `1.0` over it is `inf`, not `-inf`.
use std::hint::black_box;

fn main() {
    let a: i32 = black_box(-2);
    let r = a % 2;
    println!("{:?} {:?} {}", r as f64, 1.0 / (r as f64), r);
    let b: i8 = black_box(-9);
    println!("{:?} {:?}", (b % 3) as f64, (b % 4) as f64);
    let c: i16 = black_box(-300);
    println!("{:?} {:?}", (c % 100) as f64, (c % 7) as f64);
    let d: i64 = black_box(-4);
    println!("{:?}", (d % 2) as f64);
    let u: u32 = black_box(6);
    println!("{:?}", (u % 3) as f64);
}
