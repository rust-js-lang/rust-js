//@ run-fail: index out of bounds: the len is 2 but the index is 5
// An index in generic code of an `S: Index<usize>`, a `Vec` its caller
// gives, panics where the generic body indexes, as `Index::index` tracks
// its caller and the body doesn't: never where the caller made the
// dictionary (#2488).
use std::ops::Index;

fn nth<S: Index<usize> + ?Sized>(items: &S, at: usize) -> &S::Output {
    &items[at]
}

fn main() {
    let words = vec!["a", "b"];
    println!("{}", nth(&words, 1));
    println!("{}", nth(&words[..], 0));
    println!("{}", nth(&words, 5));
}
