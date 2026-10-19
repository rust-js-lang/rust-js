// `map(f)` of a `fmt::Result` where no writer fails, writeable's
// `write_to(sink).map(Ok)`: `Ok(f(()))`, its function called (ADR 0187).
use std::fmt::{self, Write};

fn shown(n: u32) -> Result<Result<String, u8>, fmt::Error> {
    let mut text = String::new();
    write!(text, "<{n}>").map(|()| Ok(text))
}

fn counted(out: &mut String) -> Result<usize, fmt::Error> {
    out.write_str("abc").map(|()| out.len())
}

fn main() {
    println!("{:?}", shown(7));
    let mut out = String::from("x");
    println!("{:?} {out}", counted(&mut out));
}
