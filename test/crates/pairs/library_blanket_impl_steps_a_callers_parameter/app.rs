//! A consumer's generic function given an iterator as it is, by a bound
//! that reads its value, `Clone`, calling the library's default that doesn't
//! step it, as itertools' test_core.rs's `get_esi_then_esi` does: the
//! library's impl's dictionary for its type parameter steps it by its other
//! defaults, so the function is given how to step it, though it steps
//! nothing itself (#1409).
use dep::Tools;

fn handed<I: Iterator<Item = u32> + Clone>(it: I) -> I {
    let copy = it.clone();
    copy.first_of(|rest| rest)
}

pub fn main() {
    let mut first = handed(vec![4u32, 5, 6].into_iter());
    println!("{:?} {}", first.next(), first.counted());
    let mut second = handed(1..4u32);
    println!("{:?} {}", second.next(), second.counted());
}
