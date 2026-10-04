// The bit methods num-traits' `PrimInt` implements for every integer:
// ones counted from either end, bytes swapped, bits reversed, endianness,
// and checked Euclidean division; and a float's `recip` and bytes.

fn main() {
    println!("{} {} {} {}", 0b1110_0011u8.leading_ones(), 0b1110_0011u8.trailing_ones(), (-1i16).leading_ones(), 0u64.trailing_ones());
    println!("{} {} {}", 0x1234u16.swap_bytes(), (-2i32).swap_bytes(), 0x0102_0304_0506_0708u64.swap_bytes());
    println!("{} {} {}", 1u8.reverse_bits(), (-128i8).reverse_bits(), 6u128.reverse_bits());
    println!("{} {} {} {}", 0x1234u16.to_be(), u32::from_be(1), 7i64.to_le(), i16::from_le(-5));
    println!("{:?} {:?} {:?}", 7i32.checked_div_euclid(-2), (-7i32).checked_rem_euclid(2), i8::MIN.checked_div_euclid(-1));
    println!("{:?} {:?} {:?}", 5u64.checked_div_euclid(0), (-7i64).checked_div_euclid(2), 9u128.checked_rem_euclid(4));

    println!("{} {} {}", 4.0f64.recip(), 3.0f32.recip(), 0.0f64.recip());
    println!("{:?} {:?}", 1.5f32.to_le_bytes(), (-2.0f64).to_be_bytes());
    println!("{} {}", f32::from_be_bytes([0x40, 0x49, 0x0f, 0xdb]), f64::from_le_bytes(0.1f64.to_le_bytes()));
}
