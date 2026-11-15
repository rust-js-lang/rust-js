//@ compile-fail: `capacity()` of a `Vec` std may collect in place from a source a call hides (#1926)
// A `collect()` std may make in place, in its source `Vec`'s buffer, keeps
// that buffer's capacity. Where a call hides which buffer that is, its
// result no parameter's iterator, `fresh`'s another `Vec`'s, a `capacity()`
// of what's collected, which the same body reads, is refused where it's
// compiled, not thrown where it runs (#1926; ADR 0419, amended). Collect
// from `v.into_iter()` where the capacity is read, or don't read it.
fn fresh<I: Iterator<Item = u32>>(_it: I) -> std::vec::IntoIter<u32> {
    let mut made = Vec::with_capacity(5);
    made.extend([7, 8]);
    made.into_iter()
}

fn main() {
    let mut kept = Vec::with_capacity(10);
    kept.extend([1, 2, 3]);
    let d: Vec<u32> = fresh(kept.into_iter()).map(|x| x * 2).collect();
    println!("{d:?} {}", d.capacity());
}
