//! A `#![no_std]` crate given a crate named `std` (#455), whose `std` is
//! then that crate (ADR 1054).
#![no_std]

pub fn quadruple(x: i32) -> i32 {
    std::double(std::double(x))
}

pub fn exited(code: i32) -> i32 {
    std::process::exit(code)
}

pub fn taken(mut x: i32) -> [i32; 2] {
    [std::mem::take(&mut x), x]
}

pub fn hidden(x: i32) -> i32 {
    std::hint::black_box(x)
}

pub fn same(a: i32, b: i32) -> bool {
    std::ptr::addr_eq(a, b)
}
