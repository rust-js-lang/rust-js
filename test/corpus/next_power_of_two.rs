// `next_power_of_two()` and `checked_next_power_of_two()` of unsigned
// numbers, a JS number's and a `bigint`'s: the least power of two at least
// it; past its width, `0` as release Rust wraps, and `None`. hashbrown's
// capacity (ADR 0064).
fn main() {
    let xs: [u32; 6] = [0, 1, 3, 8, 9, 3_000_000_000];
    for x in xs {
        println!("{} {:?}", x.next_power_of_two(), x.checked_next_power_of_two());
    }
    let big: u64 = 1 << 40;
    println!(
        "{} {:?} {:?}",
        (big + 1).next_power_of_two(),
        (u64::MAX).checked_next_power_of_two(),
        5u8.checked_next_power_of_two()
    );
}
