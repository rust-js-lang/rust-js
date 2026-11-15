// An `f32` literal is the `f32` Rust reads it as (ADR 0122): JS reads its
// digits as a double first, so `Math.fround` of them rounds twice, and of
// one `f32`, `7.038531e-26`, ryu's regression, lands on the `f32` beside it.
// Its digits are then the fewest whose double rounds to it.

const SMALL: f32 = 7.038531e-26;

fn main() {
    let x = 7.038531e-26f32;
    println!("{:#x} {} {:?}", x.to_bits(), x, x);
    let negative = -7.038531e-26f32;
    println!("{:#x} {negative:e}", negative.to_bits());
    println!("{}", SMALL == x);
    let parsed: f32 = "7.038531e-26".parse().unwrap();
    println!("{}", parsed == x);
    let near = [7.0385306e-26f32, 7.0385313e-26];
    println!("{:?} {:?}", near.map(f32::to_bits), near);
}
