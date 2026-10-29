// A 32-bit integer rotated by a constant is JS's shifts, `(x << 15) | (x
// >>> 17)`, as MurmurHash3's JS writes it; a `u32`'s made unsigned; by 0,
// 32 or 33 as Rust reduces it; and by a variable, the runtime's.

fn main() {
    let xs: [u32; 4] = [0, 1, 0x8000_0001, 0xdead_beef];
    for x in xs {
        println!(
            "{} {} {} {} {} {}",
            x.rotate_left(15),
            x.rotate_right(13),
            x.rotate_left(0),
            x.rotate_left(32),
            x.rotate_right(33),
            x.rotate_left(31)
        );
    }
    let ys: [i32; 4] = [0, -1, i32::MIN, -123_456_789];
    for y in ys {
        println!("{} {} {} {}", y.rotate_left(15), y.rotate_right(13), y.rotate_left(1), y.rotate_right(31));
    }
    for n in [0, 5, 32, 40] {
        println!("{} {}", 0xdead_beef_u32.rotate_left(n), (-5_i32).rotate_right(n));
    }
}
