//! A library's generic code bounded by `T: Copy` alone, which clones what
//! it's given where std's code calls `Clone`: `to_vec()`, `cloned()` and
//! `repeat`'s first copy, and copies its bits where std does: `copied()`
//! (ADR 0623, amended, #1451). A consumer's type with a hand-written
//! `Clone` has it called by each clone and by no copy.
pub fn to_vec_of<T: Copy>(v: &[T]) -> Vec<T> {
    v.to_vec()
}

pub fn cloned_of<T: Copy>(v: &[T]) -> Vec<T> {
    v.iter().cloned().collect()
}

pub fn repeated<T: Copy>(v: &[T], n: usize) -> Vec<T> {
    v.repeat(n)
}

pub fn copied_of<T: Copy>(v: &[T]) -> Vec<T> {
    v.iter().copied().collect()
}
