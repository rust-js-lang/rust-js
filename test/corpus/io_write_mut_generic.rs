// An `io::Write` given by `&mut`, `W = &mut Vec<u8>`, to generic code: its
// writes are the `Vec`'s, through the box the `&mut` is, as a `fmt::Write`
// given so writes its `String`.
use std::io::Write;

fn greet<W: Write>(mut out: W) -> std::io::Result<()> {
    out.write_all(b"hi ")?;
    write!(out, "{}", 7)?;
    out.flush()
}

fn twice<W: Write>(out: &mut W) {
    greet(&mut *out).unwrap();
    let n = out.write(b"!").unwrap();
    println!("{n}");
}

fn main() {
    let mut bytes: Vec<u8> = Vec::new();
    greet(&mut bytes).unwrap();
    twice(&mut bytes);
    println!("{}", String::from_utf8(bytes).unwrap());
}
