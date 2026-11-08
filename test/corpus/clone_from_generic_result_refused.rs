//@ compile-fail: `clone_from` of `std::result::Result<T, E>`, whose std `clone_from` is its value's
// `clone_from` of a `Result` of a type only the caller knows: std's is its
// value's where both are `Ok` or both `Err`, a `Vec`'s in place where the
// caller's `T` is one, which rust-js doesn't write yet (#927): refused, not
// `*dest = source.clone()` (#928).
fn assign<T: Clone, E: Clone>(dest: &mut Result<T, E>, source: &Result<T, E>) {
    dest.clone_from(source);
}

fn main() {
    let mut w: Result<Vec<u8>, u8> = Ok(vec![7]);
    assign(&mut w, &Ok(vec![3, 1]));
    println!("{w:?}");
}
