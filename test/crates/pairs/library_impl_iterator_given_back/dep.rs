//! A library's functions giving back an `impl Iterator` (ADR 1169, #1999):
//! its exports give JS the JS iterator, knowing its back and its length
//! where their bounds say; its consumer's Rust calls a copy of its own,
//! which gives what each reveals, as the contract's `ReturnsJs` says.
pub fn upto(n: u32) -> impl DoubleEndedIterator<Item = u32> + ExactSizeIterator {
    0..n
}

pub fn counted<T: Clone>(items: &[T]) -> impl DoubleEndedIterator<Item = T> + '_ {
    items.iter().cloned()
}

pub struct Bag(pub Vec<u32>);

impl Bag {
    pub fn odds(&self) -> impl DoubleEndedIterator<Item = u32> + '_ {
        self.0.iter().copied().filter(|x| x % 2 == 1)
    }
}
