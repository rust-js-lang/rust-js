//@ run-fail: end of range should be a character boundary
// `replace_range` inside a `char` panics with std's message (ADR 0323).

fn main() {
    let mut text = String::from("héllo");
    text.replace_range(0..2, "x");
    println!("{text}");
}
