//! A placeholder's width, fill, alignment, `+` and `#` reach a library's
//! impl that forwards by path, as native Rust's do (#1164).
use std::fmt::Write;

pub fn main() {
    println!("{:?} [{:#06x}] [{:#x}]", dep::Hexy(255), dep::Hexy(255), dep::Hexy(16));
    println!("[{:>5}] [{:<4?}] [{:+}]", dep::Shown(7), dep::Shown(8), dep::Shown(9));
    println!("{:?} {:?}", dep::Hexf(10u8), dep::Hexf((1u8, 255u8)));
    let mut full = dep::Full(4);
    println!("{:?} {}", write!(full, "{:?}", dep::Hexy(255)), full.0);
}
