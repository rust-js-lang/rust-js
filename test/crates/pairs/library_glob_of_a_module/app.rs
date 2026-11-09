//! A library's glob-re-exported module, named through the glob, as its
//! consumer writes it: `dep::support::helper` and `dep::floor_of` (#1102).
pub fn main() {
    println!("{} {}", dep::floor_of(2), dep::support::helper(3));
}
