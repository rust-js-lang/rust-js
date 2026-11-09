//! std's `write_all` of a library's `io::Write` may fail as the writer's
//! `write` does: `put` passes its error on, as native Rust's does (#1072).
use std::io::{self, Write};

fn put(w: &mut dep::Full, bytes: &[u8]) -> io::Result<()> {
    w.write_all(bytes)
}

pub fn main() {
    let mut full = dep::Full(4);
    println!("{} {}", put(&mut full, b"abc").is_ok(), full.0);
    println!("{} {}", put(&mut full, b"abc").is_err(), full.0);
}
