// An integer's checked, wrapping, overflowing and saturating methods, each
// family the same answer the others' imply: `checked_neg` is `None` where
// `overflowing_neg` says it overflowed, and `wrapping_neg` is what that gives.

fn main() {
    // Signs.
    println!("{} {} {} {}", 5i32.is_positive(), 0i8.is_positive(), (-3i64).is_negative(), 0i16.is_negative());

    // Negation, of signed and unsigned.
    for x in [5i32, 0, i32::MIN] {
        println!("{:?} {} {:?} {:?}", x.checked_neg(), x.wrapping_neg(), x.overflowing_neg(), x.checked_abs());
    }
    for x in [0u8, 1, 200] {
        println!("{:?} {} {:?}", x.checked_neg(), x.wrapping_neg(), x.overflowing_neg());
    }
    println!("{} {} {:?}", i64::MIN.wrapping_neg(), 7u64.wrapping_neg(), i64::MIN.checked_abs());
    println!("{} {} {}", i32::MIN.wrapping_abs(), (-7i8).wrapping_abs(), i8::MIN.wrapping_abs());

    // Overflowing arithmetic: what wrapping gives, and whether it wrapped.
    println!("{:?} {:?} {:?}", 250u8.overflowing_add(10), 5u32.overflowing_sub(6), i32::MAX.overflowing_mul(2));
    println!("{:?} {:?} {:?}", 3i32.overflowing_add(4), (-128i8).overflowing_sub(1), 100000i32.overflowing_mul(100000));
    println!("{:?} {:?}", u64::MAX.overflowing_add(1), i64::MIN.overflowing_mul(-1));

    // Division and remainder, where `MIN / -1` overflows.
    println!("{:?} {:?} {:?} {:?}", 7i32.checked_rem(2), 7i32.checked_rem(0), i32::MIN.checked_rem(-1), (-8i32).checked_rem(2));
    println!("{} {} {} {}", i32::MIN.wrapping_div(-1), (-7i32).wrapping_div(2), i32::MIN.wrapping_rem(-1), (-7i32).wrapping_rem(2));
    println!("{} {} {:?}", 0i32.wrapping_div(-5), 9u64.wrapping_rem(4), i64::MIN.checked_rem(-1));

    // Shifts: checked by the width, wrapping by the amount's low bits.
    let by = 33u32;
    println!("{:?} {:?} {} {}", 1i32.checked_shl(4), 1i32.checked_shl(by), 1i32.wrapping_shl(by), 255u8.wrapping_shr(9));
    println!("{:?} {:?} {}", (-16i32).checked_shr(2), 1u64.checked_shl(64), 1u64.wrapping_shl(65));

    // Powers.
    println!("{} {} {} {}", 3i32.wrapping_pow(21), 2u8.wrapping_pow(9), (-2i32).saturating_pow(31), (-2i32).saturating_pow(32));
    println!("{} {} {}", 10u32.saturating_pow(10), 2i64.saturating_pow(70), 7u8.saturating_pow(2));
    // Past the range, below it for a negative base to an odd power.
    println!("{} {} {}", (-3i32).saturating_pow(21), (-3i32).saturating_pow(22), (-2i64).saturating_pow(65));
}
