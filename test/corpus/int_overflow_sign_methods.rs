// The rest of #119: overflowing_shl/shr/abs/pow, ilog and checked_ilog of a
// base, checked_isqrt, the *_add_signed and *_add_unsigned forms,
// checked_signed_diff, and a byte's eq_ignore_ascii_case.
fn id<T>(x: T) -> T { x }
fn main() {
    println!("{:?} {:?} {:?}", id(1u32).overflowing_shl(33), id(-1i8).overflowing_shr(9), id(1u64).overflowing_shl(63));
    println!("{:?} {:?} {:?}", id(i32::MIN).overflowing_abs(), id(-5i8).overflowing_abs(), id(i64::MIN).overflowing_abs());
    println!("{:?} {:?} {:?}", id(3u8).overflowing_pow(6), id(2i64).overflowing_pow(10), id(3u64).overflowing_pow(41));
    println!("{} {} {}", id(1000u32).ilog(10), id(81u64).ilog(3), id(u128::MAX).ilog(16));
    println!("{:?} {:?} {:?}", id(0u8).checked_ilog(2), id(9u16).checked_ilog(1), id(27i32).checked_ilog(3));
    println!("{:?} {:?} {:?}", id(-4i32).checked_isqrt(), id(17i64).checked_isqrt(), id(i128::MAX).checked_isqrt());
    println!("{:?} {:?} {}", id(5u32).checked_add_signed(-6), id(5u8).wrapping_add_signed(-6), id(250u8).saturating_add_signed(10));
    println!("{:?} {:?}", id(0u64).overflowing_add_signed(-1), id(u128::MAX).checked_add_signed(-1));
    println!("{:?} {:?} {}", id(-1i32).overflowing_add_unsigned(u32::MAX), id(100i8).checked_add_unsigned(27), id(100i8).saturating_add_unsigned(200));
    println!("{} {:?}", id(i64::MIN).wrapping_add_unsigned(1), id(-1i64).checked_add_unsigned(u64::MAX));
    println!("{:?} {:?} {:?}", id(3u32).checked_signed_diff(5), id(u32::MAX).checked_signed_diff(0), id(0u64).checked_signed_diff(u64::MAX));
    println!("{} {} {}", id(b'a').eq_ignore_ascii_case(&b'A'), id(b'[').eq_ignore_ascii_case(&b'{'), id(b'z').eq_ignore_ascii_case(&b'Z'));
}
