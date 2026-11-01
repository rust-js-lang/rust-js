//@ crate-name: std
//! A program's crate named `std` (#455): an ordinary crate, whose items
//! rust-js never takes for std's, though each is named as one of std's is,
//! and does something else (ADR 1054). Only a `#![no_std]` crate can be
//! given it: another's `std` is std's.

pub fn double(x: i32) -> i32 {
    x * 2
}

/// `std::process::exit`'s path: it ends nothing.
pub mod process {
    pub fn exit(code: i32) -> i32 {
        code + 1
    }
}

/// `std::mem::take`'s path: it leaves what it's given as it is.
pub mod mem {
    pub fn take(x: &mut i32) -> i32 {
        *x * 3
    }
}

/// `std::hint::black_box`'s path: it's not what it's given.
pub mod hint {
    pub fn black_box(x: i32) -> i32 {
        x - 1
    }
}

/// `std::ptr::addr_eq`'s path, of numbers: equal ones.
pub mod ptr {
    pub fn addr_eq(a: i32, b: i32) -> bool {
        a == b
    }
}
