//! std's `write_fmt` of a library's writer may fail as the writer's
//! `write_str` does: `put` passes its error on, as native Rust's does.
use std::fmt::{self, Write};

fn put(w: &mut dep::Full, s: &str) -> fmt::Result {
    write!(w, "{s}")
}

pub fn main() {
    let mut full = dep::Full(4);
    println!("{} {}", put(&mut full, "abc").is_ok(), full.0);
    println!("{} {}", put(&mut full, "abc").is_err(), full.0);
}
