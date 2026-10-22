//@ compile-fail: rust-js does not support `len()` of a `std::iter::Peekable<std::iter::Map<std::slice::Iter<
// A `Peekable`'s or a `Fuse`'s hint is its struct's, of what it holds: a
// `map` made here is a JS iterator, which knows nothing of its length, so
// `len()` and `size_hint()` of one holding it are refused (#154). Of an
// array's iterator, a range or the crate's own, each is its own
// (peekable_range_len.rs).
fn main() {
    let v = vec![1, 2, 3];
    let p = v.iter().map(|x| x * 2).peekable();
    println!("{}", p.len());
}
