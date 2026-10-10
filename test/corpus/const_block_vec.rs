//@ compile-fail: rust-js does not support constants of type `std::vec::Vec<i32>` yet
// A `const { Vec::new() }` is made afresh at each use in Rust; one JS array
// would be every use's, so it's refused.
fn main() {
    let mut rows: [Vec<i32>; 2] = [const { Vec::new() }; 2];
    rows[0].push(1);
    println!("{rows:?}");
}
