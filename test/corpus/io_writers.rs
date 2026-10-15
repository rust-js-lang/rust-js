// A writer of bytes, `io::Write` (ADR 0392): generic code, a `W: Write`, is
// given its dictionary, and a `dyn Write` is the pair of it and its writer;
// a `Vec<u8>`'s bytes are added to it, a standard stream's written, and a
// write never fails (ADR 0132). Where the writer's known, it's in place.
use std::io::{self, Write};

fn render<W: Write>(out: &mut W, n: u32) -> io::Result<()> {
    writeln!(out, "n = {n}")?;
    out.write_all(b"bytes\n")?;
    out.flush()
}

fn shout(out: &mut dyn Write, text: &str) {
    write!(out, "{}!", text.to_uppercase()).unwrap();
    out.write_all(b"\n").ok().expect("a write never fails");
}

fn main() {
    let mut buf: Vec<u8> = Vec::new();
    render(&mut buf, 1).unwrap();
    shout(&mut buf, "héllo");
    write!(buf, "{}", 7).unwrap();
    buf.write_all(b"\n").unwrap();
    print!("{}", String::from_utf8(buf.clone()).unwrap());
    println!("{}", buf.len());

    render(&mut io::stdout(), 2).unwrap();
    shout(&mut io::stdout(), "stdout");
    let mut boxed: Box<dyn Write> = Box::new(io::stdout());
    shout(&mut boxed, "boxed");
    let mut into_vec: Box<dyn Write> = Box::new(Vec::new());
    shout(&mut *into_vec, "vec");
    let written = io::stdout().write(b"wrote\n").unwrap();
    println!("{written}");
    io::stdout().write_all("é whole\n".as_bytes()).unwrap();
    // A character split between two writes, and a byte that's no text.
    io::stdout().write_all(&[0xC3]).unwrap();
    io::stdout().write_all(&[0xA9, b'\n', 0xFF, b'\n']).unwrap();
    let mut err = io::stderr();
    writeln!(err, "to stderr").unwrap();
    shout(&mut err, "err");
}
