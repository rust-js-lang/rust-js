//@ crate-name: alloc
//! A program's crate named `alloc` (#455): an ordinary crate, whose items
//! rust-js never takes for std's, though each is named as one of std's is,
//! and does something else (ADR 1052).

/// `alloc::fmt::format`'s name: a text of its own.
pub mod fmt {
    pub fn format(text: &str) -> String {
        format!("<{text}>")
    }
}

/// `alloc::string::String`'s path, of a type of its own.
pub mod string {
    #[derive(Debug, Clone, PartialEq)]
    pub struct String(pub i32);
}

pub fn named(value: &dyn std::any::Any) -> &'static str {
    if value.downcast_ref::<std::string::String>().is_some() {
        "std's"
    } else if value.downcast_ref::<string::String>().is_some() {
        "alloc's"
    } else {
        "neither"
    }
}
