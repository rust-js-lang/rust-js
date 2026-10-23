// A JS string's `length` is at most 2^31 - 1 units, as every engine's is,
// so the sum of two can't pass `u32`'s most and is JS's own `+` (ADR 0682),
// as React's ReactSerializationErrors writes `str.length + substr.length`.
// A third keeps its wrap, as does a `- 1`. The binding is the program's own,
// counting UTF-16 units natively as JS's `length` does.
use std::hint::black_box;

#[cfg_attr(rust_js, rust_js::link_name = "get length")]
fn length(this: &str) -> u32 {
    this.encode_utf16().count() as u32
}

fn short(str: &str, substr: &str) -> bool {
    length(substr) < 10 && length(str) + length(substr) < 40
}

fn total(a: &str, b: &str) -> u32 {
    length(a) + length(b)
}

fn three(a: &str, b: &str, c: &str) -> u32 {
    length(a) + length(b) + length(c)
}

fn last(s: &str) -> u32 {
    length(s) - 1
}

fn main() {
    let long = "x".repeat(black_box(36));
    for (str, substr) in [("hello", "lo"), (long.as_str(), "four"), ("", "ten units!"), ("😀", "é")] {
        println!("{} {}", short(str, substr), total(str, substr));
    }
    println!("{} {}", three("a", "bc", "😀"), last(black_box("abc")));
}
