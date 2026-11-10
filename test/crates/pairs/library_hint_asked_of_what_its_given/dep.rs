//! A library's generic function that asks the hint of what it's given,
//! whose body a consumer can't see.
pub fn hint<I: Iterator>(i: I) -> (usize, Option<usize>) {
    i.size_hint()
}
