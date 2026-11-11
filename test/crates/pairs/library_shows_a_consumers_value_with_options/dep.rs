//! A library's generic functions that show what they're given with a
//! `Formatter`'s options, `{:x?}`, `{:#?}` and `{:#x?}`, and one that gives
//! what it's given on to another: its manifest says which type parameters
//! each shows so, and its consumer's writers of the types it gives them
//! take those options (#1331).
use std::fmt::Debug;

pub fn hex<T: Debug>(held: &T) -> String {
    format!("{:x?}", held)
}

pub fn pretty<T: Debug>(held: &T) -> String {
    format!("{:#?}", held)
}

pub fn pretty_hex<T: Debug>(held: &T) -> String {
    format!("{:#x?}", held)
}

/// Shows nothing itself: gives `T` to `pretty`, wrapped.
pub fn labelled<T: Debug>(label: &str, held: &T) -> String {
    format!("{label}: {}", pretty(&(held, 1u8)))
}
