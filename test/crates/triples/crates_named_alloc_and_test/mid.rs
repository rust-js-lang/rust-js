//@ crate-name: test
//! A program's crate named `test`, libtest's name (#455), calling one
//! named `alloc`: neither is the sysroot's (ADR 1052).

/// libtest's `test::black_box`'s name: not what it's given.
pub fn black_box(x: i32) -> i32 {
    x * 10
}

pub fn wrapped(text: &str) -> String {
    alloc::fmt::format(text)
}
