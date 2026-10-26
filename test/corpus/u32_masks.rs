// A `u32`'s `&` of a constant below 2^31 is JS's `&` as it is, `n & 0xff`:
// bit 31 cleared, its signed result is the value. Of a mask with bit 31, or
// of two variables, it's made unsigned, `>>> 0`.

fn low(n: u32) -> u32 {
    n & 1
}

fn byte(n: u32) -> u32 {
    n & 0xff
}

fn top(n: u32) -> u32 {
    n & 0x8000_0000
}

fn both(a: u32, b: u32) -> u32 {
    a & b
}

fn set_bits(mut n: u32) -> u32 {
    let mut count = 0;
    while n != 0 {
        count += n & 1;
        n >>= 1;
    }
    count
}

fn main() {
    for n in [0, 1, 0xff, 0x8000_0001, u32::MAX] {
        println!(
            "{} {} {} {} {}",
            low(n),
            byte(n),
            top(n),
            both(n, u32::MAX),
            set_bits(n)
        );
        println!("{}", low(n) == 1 && byte(n) > 0x7f);
    }
}
