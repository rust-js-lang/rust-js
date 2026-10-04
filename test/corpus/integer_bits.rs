// An integer's bits rotated and its bytes in either order, and a float's
// bits: what hashes, checksums and binary formats are made of.

// FNV-1a, as small hashes are written: a `wrapping_mul` and a `^`.
fn fnv1a(text: &str) -> u32 {
    let mut hash = 0x811c9dc5u32;
    for byte in text.bytes() {
        hash ^= u32::from(byte);
        hash = hash.wrapping_mul(0x01000193);
    }
    hash
}

// xorshift, as small random number generators are written.
fn xorshift(mut x: u64) -> u64 {
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    x
}

fn main() {
    // Rotated, by less and more than the width, signed and unsigned.
    println!("{} {} {} {}", 1u32.rotate_left(31), 0x80000001u32.rotate_right(1), 1u8.rotate_left(9), 0x81u8.rotate_right(4));
    println!("{} {} {}", (-2i32).rotate_left(1), i8::MIN.rotate_right(7), (-1i16).rotate_left(3));
    println!("{} {}", 1u64.rotate_left(63), 0x8000000000000001u64.rotate_right(4));
    println!("{}", (-2i64).rotate_left(1));

    // Bytes, in each order, and back.
    println!("{:?} {:?} {:?}", 0x01020304u32.to_be_bytes(), 0x01020304u32.to_le_bytes(), 0x0102u16.to_ne_bytes());
    println!("{:?} {:?} {:?}", (-2i32).to_be_bytes(), (-1i8).to_le_bytes(), 258u64.to_be_bytes());
    println!("{:?}", (-3i64).to_le_bytes());
    println!(
        "{} {} {} {}",
        u32::from_be_bytes([1, 2, 3, 4]),
        i16::from_le_bytes([0xfe, 0xff]),
        u64::from_be_bytes([0, 0, 0, 0, 0, 0, 1, 2]),
        i64::from_ne_bytes((-7i64).to_ne_bytes())
    );

    // A float's bits, and back.
    println!("{} {} {}", 1.5f64.to_bits(), (-0.0f64).to_bits(), f64::INFINITY.to_bits());
    println!("{} {} {}", 1.5f32.to_bits(), f32::from_bits(0x40490fdb), f64::from_bits(0x400921fb54442d18));
    println!("{} {}", f64::from_bits(1.25f64.to_bits()), f32::MIN_POSITIVE.to_bits());

    println!("{} {}", fnv1a("hello"), xorshift(88172645463325252));
}
