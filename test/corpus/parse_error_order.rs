// Parsing an integer stops at the first digit that's wrong, as Rust's does:
// `"999x"` overflows a `u8` at its third digit, before its `x` is read, and
// `"25x"` doesn't, so its `x` is the error.

fn main() {
    // The last two are an `i64`'s and a `u64`'s `"26x"`: ten times what's
    // before the `x` is too large, but the `x` is read first.
    for s in [
        "999x",
        "26x",
        "25x",
        "-129x",
        "x300",
        "300",
        "99999999999999999999x",
        "-99999999999999999999",
        "922337203685477581x",
        "1844674407370955162x",
    ] {
        println!(
            "{s}: {:?} {:?} {:?} {:?}",
            s.parse::<u8>().map_err(|e| e.to_string()),
            s.parse::<i8>().map_err(|e| e.to_string()),
            s.parse::<i64>().map_err(|e| e.to_string()),
            s.parse::<u64>().map_err(|e| e.to_string())
        );
    }
}
