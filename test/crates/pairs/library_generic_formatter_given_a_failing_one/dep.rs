//! A library's generic `fmt::Write` writers and formatters of a generic
//! `Display`, none of whose own writes fail: its consumer's writer or value
//! may (#1392).
use std::fmt::{self, Display, Write};

pub fn render<W: Write>(out: &mut W, n: u32) -> fmt::Result {
    write!(out, "<{n}>")
}

pub fn rendered<W: Write>(out: &mut W, n: u32) -> bool {
    write!(out, "<{n}>").is_ok()
}

pub fn show_into<T: Display>(out: &mut String, t: &T) -> fmt::Result {
    write!(out, "[{t}]")
}

pub fn shown<T: Display>(t: &T) -> String {
    let mut out = String::new();
    match write!(out, "[{t}]") {
        Ok(()) => out,
        Err(_) => format!("refused after {out:?}"),
    }
}
