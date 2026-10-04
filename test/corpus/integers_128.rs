// `u128` and `i128`: BigInts wrapped to 128 bits, as `u64` and `i64` are to
// 64 (ADR 0086), from the literals at their ends to casts, methods, parsing
// and formatting, as num-traits, num-conv and serde implement them.

fn fnv(bytes: &[u8]) -> u128 {
    let mut hash: u128 = 0x6c62272e07bb014262b821756295c58d;
    for &b in bytes {
        hash ^= b as u128;
        hash = hash.wrapping_mul(0x0000000001000000000000000000013B);
    }
    hash
}

fn main() {
    let max = u128::MAX;
    let (min, top) = (i128::MIN, i128::MAX);
    println!("{} {} {}", max, min, top);
    println!("{:x} {:#b} {:?}", max, 5u128, -7i128);
    println!("{}", fnv(b"rust-js"));

    // Arithmetic, wrapped as release Rust wraps it.
    let big: u128 = 1 << 100;
    println!("{} {} {}", big + 1, big.wrapping_mul(big), max.wrapping_add(2));
    println!("{} {} {}", top.wrapping_add(1), min.wrapping_sub(1), -(top / 3));
    println!("{} {} {}", big >> 99, (big << 27) >> 100, max % 1_000_000_007);
    println!("{} {}", min / 7, min % 7);

    // Methods.
    println!("{:?} {:?} {:?}", max.checked_add(1), big.checked_mul(4), top.checked_neg());
    println!("{} {} {}", max.saturating_add(9), min.saturating_sub(1), 3u128.pow(80));
    println!("{} {} {}", big.leading_zeros(), big.trailing_zeros(), max.count_ones());
    println!("{} {} {}", min.unsigned_abs(), (-5i128).abs(), (-5i128).rem_euclid(3));
    println!("{} {}", big.is_power_of_two(), max.max(big));

    println!("{:?} {:?} {:?}", top.overflowing_add(1), big.checked_shl(130), 1u128.checked_shl(100));
    println!("{} {} {}", big.rotate_left(30), (-2i128).rotate_right(1), big.count_zeros());
    println!("{:?} {}", 258u128.to_le_bytes(), u128::from_be_bytes([1; 16]));
    println!("{} {}", i128::from_le_bytes((-3i128).to_le_bytes()), big as f32);

    // Casts.
    let x: u64 = u64::MAX;
    println!("{} {} {}", x as u128 * 3, (max as u64), (-1i128) as u128);
    println!("{} {} {}", big as f64, 1e40f64 as u128, (-1e40f64) as i128);
    println!("{} {}", (big + 300) as u8, u128::from(7u32) + u128::from(x));
    println!("{:?} {:?} {:?}", u64::try_from(big).ok(), i128::try_from(max).ok(), u8::try_from(200u128).ok());

    // Parsing.
    println!("{:?}", "340282366920938463463374607431768211455".parse::<u128>());
    println!("{:?}", "340282366920938463463374607431768211456".parse::<u128>().is_err());
    println!("{:?}", "-170141183460469231731687303715884105728".parse::<i128>());
    println!("{:?}", u128::from_str_radix("ff", 16));

    // Matching and ordering.
    let size = match big {
        0 => "zero",
        1..=0xffff_ffff_ffff_ffff => "fits in 64",
        _ => "wider",
    };
    let mut values = vec![max, 0, big, 1];
    values.sort();
    println!("{} {:?} {}", size, values, values[..3].iter().sum::<u128>());
}
