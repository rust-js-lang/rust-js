//@ run-fail: called `Result::unwrap()` on an `Err` value: LayoutError
// A collect by generic code into an `Arc<[T]>`, whose caller says its
// iterator is `TrustedLen`, past what a `Layout` of the slice holds
// panics where std's `Arc::<[T]>::allocate_for_slice` unwraps its
// `LayoutError`, `sync.rs`'s line, as std's hook says (#2489).
use std::sync::Arc;

fn made<C: FromIterator<u64>, I: Iterator<Item = u64>>(items: I) -> C {
    items.collect()
}

fn main() {
    let small: Arc<[u64]> = made(0..3u64);
    println!("{small:?}");
    let huge: Arc<[u64]> = made((0..usize::MAX).map(|i| i as u64));
    println!("{}", huge.len());
}
