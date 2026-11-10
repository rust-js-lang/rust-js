//! A library's generic functions that make an adapter of what they're
//! given and ask its hint, whose callers, a consumer's, the library can't
//! see.
pub fn hint<I: Iterator>(it: I) -> (usize, Option<usize>) {
    it.size_hint()
}

pub fn doubled<I: Iterator<Item = u32>>(it: I) -> (usize, Option<usize>) {
    hint(it.map(|x| x * 2))
}

pub fn first_two<I: Iterator<Item = u32>>(it: I) -> (usize, Option<usize>) {
    hint(it.take(2).map(|x| x + 1))
}
