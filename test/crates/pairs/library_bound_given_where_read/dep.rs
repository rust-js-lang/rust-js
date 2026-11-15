//! A library's generic functions, one of a bound it never reads, which its
//! contract lists so a consumer gives it no dictionary, and one it reads
//! (#1915).
use std::fmt::Debug;

/// What it's given, labeled: its `Debug` is never read.
pub fn labeled<T: Debug>(value: T, label: &str) -> (T, String) {
    (value, label.to_string())
}

/// What it's given, shown: its `Debug` is read.
pub fn shown<T: Debug>(value: &T) -> String {
    format!("{value:?}")
}
