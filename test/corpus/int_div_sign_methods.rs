// overflowing_div, overflowing_rem, saturating_div, is_multiple_of,
// cast_unsigned and cast_signed, of numbers and BigInts alike.
fn id<T>(x: T) -> T { x }
fn main() {
    println!("{:?} {:?} {:?}", id(i8::MIN).overflowing_div(-1), id(i64::MIN).overflowing_rem(-1), id(-7i128).overflowing_div(2));
    println!("{:?} {:?}", id(u64::MAX).overflowing_div(3), id(9u8).overflowing_rem(4));
    println!("{} {} {}", id(i64::MIN).saturating_div(-1), id(i128::MIN).saturating_div(-1), id(-9i16).saturating_div(4));
    println!("{} {} {} {}", id(u128::MAX).is_multiple_of(5), id(0u8).is_multiple_of(0), id(7u16).is_multiple_of(0), id(48usize).is_multiple_of(16));
    println!("{} {} {}", id(i8::MIN).cast_unsigned(), id(-2i64).cast_unsigned(), id(i128::MIN).cast_unsigned());
    println!("{} {} {}", id(255u8).cast_signed(), id(u64::MAX - 1).cast_signed(), id(1u32 << 31).cast_signed());
}
