// A chain of generic code that starts on the iterator it's given, and maps
// past a link that isn't std's compiled struct: `src.chain(v).map(f)`'s
// `map` is given the chain made in place, so it's the hand-written form
// too, and folds from its back what native Rust does (#1780). One that
// maps the source itself is std's compiled `Map`.

use std::cell::Cell;

thread_local! {
    static DROPS: Cell<u32> = const { Cell::new(0) };
}

#[derive(Debug)]
struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        DROPS.with(|drops| drops.set(drops.get() + 1));
    }
}

fn drops() -> u32 {
    DROPS.with(Cell::get)
}

fn chained<I: DoubleEndedIterator<Item = D>>(src: I) -> u32 {
    let tail = vec![D(6), D(7)];
    src.chain(tail).map(|d| d.0 * 10).rfold(0, |sum, x| sum * 2 + x)
}

fn taken<I: DoubleEndedIterator<Item = D> + ExactSizeIterator>(src: I) -> u32 {
    src.take(3).map(|d| d.0 + 1).rfold(0, |sum, x| sum * 2 + x)
}

fn stepped<I: DoubleEndedIterator<Item = D> + ExactSizeIterator>(src: I) -> u32 {
    src.step_by(2).map(|d| d.0).rfold(0, |sum, x| sum * 3 + x)
}

fn flattened<I: DoubleEndedIterator<Item = Vec<D>>>(src: I) -> Vec<u32> {
    src.flatten().map(|d| d.0 * 3).rev().collect()
}

fn mapped<I: DoubleEndedIterator<Item = D> + ExactSizeIterator>(src: I) -> Vec<(usize, u32)> {
    src.map(|d| d.0 + 100).enumerate().rev().collect()
}

fn ds() -> Vec<D> {
    (1..6).map(D).collect()
}

fn main() {
    println!("chained {} after {}", chained(ds().into_iter()), drops());
    println!("taken {} after {}", taken(ds().into_iter()), drops());
    println!("stepped {} after {}", stepped(ds().into_iter()), drops());
    let nested = vec![vec![D(1), D(2)], vec![], vec![D(3)]];
    println!("flattened {:?} after {}", flattened(nested.into_iter()), drops());
    println!("mapped {:?} after {}", mapped(ds().into_iter()), drops());
}
