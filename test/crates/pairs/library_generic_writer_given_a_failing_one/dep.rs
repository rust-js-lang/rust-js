//! A library's generic writers, none of whose own writes fail: its
//! consumer's writer may, a file opened to read or a writer of its own, and
//! each passes on or tests what it gave, as Rust's does (#1392).
use std::io::{self, Write};

pub fn emit<W: Write>(out: &mut W) -> io::Result<()> {
    out.write_all(b"x")
}

pub fn emit_twice<W: Write>(out: &mut W) -> io::Result<()> {
    out.write_all(b"x")?;
    println!("between");
    out.write_all(b"y")
}

pub fn emitted<W: Write>(out: &mut W) -> bool {
    out.write_all(b"x").is_ok()
}

pub fn emit_ignoring<W: Write>(out: &mut W) -> u32 {
    let _ = out.write_all(b"x");
    7
}

pub fn emit_or<W: Write>(out: &mut W) -> String {
    match out.write_all(b"x") {
        Ok(()) => String::from("ok"),
        Err(e) => format!("err {:?}", e.kind()),
    }
}

/// Through a helper of its own, which its consumer's copy calls too.
pub fn emit_through<W: Write>(out: &mut W) -> bool {
    emitted(out)
}
