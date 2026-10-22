// A `u128` or `i128` past `f32::MAX` once rounded to an `f32`'s 24 bits is
// infinity, as Rust's cast makes it; one below rounds to `f32::MAX`.
use std::hint::black_box;

fn main() {
    let max = black_box(u128::MAX);
    println!("{} {}", max as f32, (max as f32).is_infinite());
    let midpoint = black_box(u128::MAX - (1 << 103) + 1);
    let below = black_box(u128::MAX - (1 << 104));
    println!("{} {}", midpoint as f32, below as f32 == f32::MAX);
    let least = black_box(i128::MIN);
    println!("{} {}", least as f32, -(least as f32) == 2f32.powi(127));
}
