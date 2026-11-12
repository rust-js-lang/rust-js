//! A library's writers shown in a crate that catches panics, inside its
//! own `Vec`s and `Result`s and through its generic code: another crate's
//! writer is shown whole, as it's compiled there, and never refused as one
//! that may stop part-way (#1496).
use std::panic;

/// Shown by a `println!` of whatever it's given, the consumer's own.
fn listed<T: std::fmt::Debug>(values: Vec<T>) {
    println!("listed {values:?}");
}

pub fn main() {
    let caught = panic::catch_unwind(|| 1);
    println!("{:?} {}", vec![dep::Label("a"), dep::Label("b")], caught.is_ok());
    let modes: Result<Vec<dep::Mode>, String> = Ok(vec![dep::Mode::Off, dep::Mode::On(3)]);
    println!("{modes:?}");
    dep::shown(dep::Label("c"));
    listed(vec![dep::Mode::On(4)]);
}
