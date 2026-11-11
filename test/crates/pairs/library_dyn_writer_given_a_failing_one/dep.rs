//! A library's writers through a `dyn`, which no consumer can compile for
//! its own: a write through one may fail, whatever the library's own do
//! (#1392).
use std::fmt::Display;
use std::io::{self, Write};

pub fn emit(out: &mut dyn Write) -> io::Result<()> {
    out.write_all(b"x")
}

pub fn emitted(out: &mut dyn Write) -> bool {
    out.write_all(b"x").is_ok()
}

/// Through a type parameter, its consumer's to compile where what it gives
/// may fail: written as one that never does.
pub fn joined<T: Display>(items: &[T]) -> String {
    items.iter().map(|item| item.to_string()).collect::<Vec<_>>().join(",")
}
