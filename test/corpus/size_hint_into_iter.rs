// `size_hint()` of a generic `I::IntoIter`, smallvec's `extend`'s: as its JS
// value tells it, as a type parameter's is (ADR 0170), here a lazy one's,
// `(0, None)`, as native Rust's is.
fn hint<I: IntoIterator<Item = u32>>(items: I) -> (usize, Option<usize>) {
    let it = items.into_iter();
    it.size_hint()
}

fn main() {
    let mut n = 0;
    let counted = std::iter::from_fn(move || {
        n += 1;
        (n < 3).then_some(n)
    });
    println!("{:?}", hint(counted));
}
