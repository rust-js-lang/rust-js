// A wide integer literal cast to a narrower type is folded exactly, as the
// BigInt it is, never through a JS number, which past 2^53 isn't the
// literal: `18446744073709551615_u64 as u32` is `4294967295`, where through
// `18446744073709552000 >>> 0` it was 0 (#1380, found by Rustlantis). As
// rustc gives it, a `let` binding the cast of a constant, a negation of one
// folded first; a `u128` past `i128::MAX` is cast as it runs.
fn main() {
    let a = (-57850311006394435500531941819513103853_i128) as i32;
    let c = 13410778826400528353_u64 as i32;
    let d = (-9223372036854775807_i64) as u8;
    let e = 18446744073709551615_u64 as u32;
    let f = 4706315301130638003_i64 as u32;
    let g = -(-18446744073709551615_i128) as u32;
    let h = 340282366920938463463374607431768211455_u128 as u16;
    let i = (-9007199254740993_i64) as i16;
    let j = 9007199254740993_u64 as f64;
    println!("{a} {c} {d} {e} {f} {g} {h} {i} {j}");
}
