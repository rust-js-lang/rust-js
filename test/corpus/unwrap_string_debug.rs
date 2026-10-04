//@ run-fail: called `Result::unwrap()` on an `Err` value: "bell \u{7}, tab \t, quote \""
// A string error shows as Rust's `{:?}` escapes it, not as JSON does.

fn main() {
    let r: Result<u8, String> = Err("bell \u{7}, tab \t, quote \"".to_string());
    println!("{}", r.unwrap());
}
