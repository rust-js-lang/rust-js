//@ compile-fail: a `&mut` to part of a slice
// A slice pattern's rest is a copy, `xs.slice(1)`: a `&mut` to it, which
// would write the copy, not the slice, is refused.
fn main() {
    let mut v = [1, 2, 3];
    if let [first, rest @ ..] = &mut v {
        *first += 1;
        rest[0] = 9;
    }
    println!("{v:?}");
}
