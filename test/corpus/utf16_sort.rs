// React's setToSortedString, `array.sort().join(', ')`: JS's `sort()`
// orders strings by their UTF-16 units, which Rust writes `sort_by(|a, b|
// a.encode_utf16().cmp(b.encode_utf16()))`, JS's `sort()` itself. It
// differs from Rust's `sort()`, by code point, `sort($cmp)`, where two
// strings first differ at a character past U+FFFF, two units from
// U+D800, and one from U+E000 to U+FFFF (ADR 0945).
fn main() {
    let words = ["b\u{1F600}", "b\u{FF61}", "a", "b\u{E000}", "", "b\u{10000}", "ab", "B", "b\u{FFFF}", "ca"];

    let mut by_points = words.to_vec();
    by_points.sort();
    println!("{by_points:?}");

    let mut by_units = words.to_vec();
    by_units.sort_by(|a, b| a.encode_utf16().cmp(b.encode_utf16()));
    println!("{by_units:?}");

    let mut owned: Vec<String> = words.iter().map(|w| w.to_string()).collect();
    owned.sort_by(|a, b| a.encode_utf16().cmp(b.encode_utf16()));
    println!("{}", owned.join(", "));

    let mut unstable: Vec<String> = words.iter().rev().map(|w| w.to_string()).collect();
    unstable.sort_unstable_by(|a, b| a.encode_utf16().cmp(b.encode_utf16()));
    println!("{unstable:?}");
}
