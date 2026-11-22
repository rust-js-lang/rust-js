// A library that prints whole lines alone, each its own `println!`, which
// rust-js writes as `console.log` (ADR 0087): called between its app's
// `print!` and that line's newline, natively one `LineWriter` holds both, so
// the library's line ends the app's (#2320).
pub fn line(n: u32) {
    println!("lib {n}");
}

pub fn lines() {
    println!("first");
    println!("second");
}
