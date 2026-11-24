//! A library's generic `B::from_iter(it)`, the `collect()` it is, which
//! its consumer compiles itself: what the consumer gives it is made knowing
//! its hint, as of a `collect()`, through a bound of `IntoIterator` (#2463).
use std::collections::VecDeque;
pub fn vec_of<I: IntoIterator<Item = u32>>(it: I) -> Vec<u32> {
    Vec::from_iter(it)
}
pub fn deque_of<I: IntoIterator<Item = u32>>(it: I) -> VecDeque<u32> {
    VecDeque::from_iter(it)
}
pub fn vec_of_refs<'a, I: IntoIterator<Item = &'a u32>>(it: I) -> Vec<&'a u32> {
    Vec::from_iter(it)
}
