// A type parameter's write whose result is tested, in a crate whose writes
// may fail: `is_ok()`, `match` and `let _ =` of its dictionary's write see
// the writer's error, as Rust's do, an `io::Write`'s and a `fmt::Write`'s
// alike (ADR 0187, amended).
use std::fmt;
use std::io::{self, Write};

fn emit<W: Write>(out: &mut W) -> io::Result<()> {
    out.write_all(b"x")
}

fn emitted<W: Write>(out: &mut W) -> bool {
    out.write_all(b"x").is_ok()
}

fn emit_ignoring<W: Write>(out: &mut W) -> u32 {
    let _ = out.write_all(b"x");
    7
}

fn emit_or<W: Write>(out: &mut W) -> String {
    match out.write_all(b"x") {
        Ok(()) => String::from("ok"),
        Err(e) => format!("err {:?} {e}", e.kind()),
    }
}

fn rendered<W: fmt::Write>(out: &mut W, n: u32) -> bool {
    write!(out, "<{n}>").is_ok()
}

struct Refusing(u8);

impl Write for Refusing {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if self.0 == 0 {
            return Err(io::Error::other("refused"));
        }
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl fmt::Write for Refusing {
    fn write_str(&mut self, _: &str) -> fmt::Result {
        if self.0 == 0 {
            return Err(fmt::Error);
        }
        Ok(())
    }
}

fn main() {
    let mut refusing = Refusing(0);
    let mut taking = Refusing(1);
    println!("{:?}", emit(&mut refusing).map_err(|e| e.to_string()));
    println!("{} {}", emitted(&mut refusing), emitted(&mut taking));
    println!("{}", emit_ignoring(&mut refusing));
    println!("{} {}", emit_or(&mut refusing), emit_or(&mut taking));
    println!("{} {}", rendered(&mut refusing, 1), rendered(&mut taking, 2));
    let mut bytes: Vec<u8> = Vec::new();
    println!("{} {} {bytes:?}", emitted(&mut bytes), emit_or(&mut bytes));
    let mut text = String::new();
    println!("{} {text}", rendered(&mut text, 3));
}
