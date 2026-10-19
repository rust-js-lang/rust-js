// `unchecked_add`, `_sub` and `_mul`, a promise they don't overflow, as
// deranged's ranged integers write them, and `saturating_neg` and
// `saturating_abs`, `MIN`'s `MAX`.

fn main() {
    // SAFETY: none of these overflow.
    let (a, b, c) = unsafe {
        (
            200u8.unchecked_add(55),
            (-7i32).unchecked_sub(3),
            3_000_000_000u64.unchecked_mul(3),
        )
    };
    let big = unsafe { i128::MAX.unchecked_sub(1) };
    println!("{a} {b} {c} {big}");
    println!(
        "{} {} {} {} {}",
        i8::MIN.saturating_neg(),
        5i8.saturating_neg(),
        i32::MIN.saturating_abs(),
        (-9i32).saturating_abs(),
        7i32.saturating_abs()
    );
    println!(
        "{} {} {}",
        i64::MIN.saturating_neg(),
        i128::MIN.saturating_abs(),
        (-4i64).saturating_abs()
    );
}
