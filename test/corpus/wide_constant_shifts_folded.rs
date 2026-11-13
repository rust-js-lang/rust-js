// A 64-bit or 128-bit constant made of a shift is folded to its value,
// wrapped as Rust wraps it: `1 << 40` is `1099511627776n`, not
// `BigInt.asIntN(64, 1n << 40n)`, and `1 << 63` of an `i64` its minimum
// (#1729). A 32-bit one's shift is JS's own, `1 << 20`, of 32 bits as
// Rust's is: `((1 << 31) >> 1) + 1` of an `i32` is negative.

fn main() {
    let big: i64 = 1 << 40;
    let top: i64 = 1 << 63;
    let wide: u128 = 1 << 100;
    let unsigned: u64 = (1 << 40) + 3;
    let negative: i64 = -(1 << 40) >> 3;
    let masked: i32 = 1 << 20;
    let half: i32 = ((1 << 31) >> 1) + 1;
    println!("{big} {top} {wide} {unsigned} {negative} {masked} {half}");
    let shift = std::hint::black_box(40);
    let runtime: i64 = 1 << shift;
    println!("{}", runtime == big);
}
