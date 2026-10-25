// A `Peekable`'s or a `Fuse`'s hint is its struct's, of what it holds: a
// `map` it holds is std's `Map` (ADR 0871, amended), which knows its
// source's, so `len()` and `size_hint()` of one holding it are Rust's,
// as of an array's iterator, a range or the crate's own
// (peekable_range_len.rs). Refused once (#154), as a JS iterator `map`
// knew nothing of its length.
fn main() {
    let v = vec![1, 2, 3];
    let p = v.iter().map(|x| x * 2).peekable();
    println!("{} {:?}", p.len(), p.size_hint());
}
