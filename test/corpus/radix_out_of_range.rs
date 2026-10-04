//@ run-fail: from_ascii_radix: radix must lie in the range `[2, 36]` - found 37
// A radix past 36 has no digit for 36.
fn main() {
    for radix in [2, 36, 37] {
        println!("{:?}", u32::from_str_radix("11", radix));
    }
}
