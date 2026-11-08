//! A `#![no_std]` library's generic functions, given this crate's types.
pub fn main() {
    println!("{:?}", dep::some(3));
    println!("{:?}", dep::all_some([Some(()), None].into_iter()));
    println!("{:?}", dep::all_some([Some(())].into_iter()));
}
