// `overflowing_div_euclid` and `overflowing_rem_euclid`: what their
// `wrapping_` gives, and whether it wrapped, `MIN` by `-1`'s (#940).
use std::hint::black_box;

fn main() {
    let min = black_box(i8::MIN);
    println!("{:?} {:?}", min.overflowing_div_euclid(-1), min.overflowing_rem_euclid(-1));
    println!("{:?} {:?}", (-7_i32).overflowing_div_euclid(black_box(2)), (-7_i32).overflowing_rem_euclid(black_box(2)));
    println!("{:?} {:?}", 7_i64.overflowing_div_euclid(black_box(-2)), 7_i64.overflowing_rem_euclid(black_box(-2)));
    println!("{:?}", black_box(i128::MIN).overflowing_div_euclid(-1));
    println!("{:?} {:?}", black_box(u64::MAX).overflowing_div_euclid(7), black_box(200_u8).overflowing_rem_euclid(7));
    let zero = std::panic::catch_unwind(|| black_box(5_u32).overflowing_div_euclid(black_box(0)));
    println!("{}", zero.is_err());
}
