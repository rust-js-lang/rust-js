//@ compile-fail: `capacity()` of a `Vec` std may collect in place from a source a call hides (#1926)
// A `collect()` std may make in place, in its source `Vec`'s buffer, keeps
// that buffer's capacity, 10 here. Where a call hides the source,
// `opaque(kept.into_iter())`, rust-js can't tell which buffer it is, so a
// `capacity()` of what's collected, which the same body reads, is refused
// where it's compiled, not thrown where it runs (#1926; ADR 0419, amended).
// Collect from `kept.into_iter()` where the capacity is read, or don't read
// it.
fn opaque<I: Iterator<Item = u32> + ExactSizeIterator>(it: I) -> impl Iterator<Item = u32> + ExactSizeIterator {
    it
}

fn main() {
    let mut kept = Vec::with_capacity(10);
    kept.extend([1, 2, 3]);
    let d: Vec<u32> = opaque(kept.into_iter()).map(|x| x * 2).collect();
    println!("{d:?} {}", d.capacity());
}
