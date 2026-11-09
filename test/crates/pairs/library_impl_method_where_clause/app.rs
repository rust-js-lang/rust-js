//! A consumer calling the library's impl's own method, directly and
//! through a generic caller's dictionary (#1080).
use dep::{Countdown, Keeping, Wrapper};

fn held_through<K: Keeping>(keeping: K) -> usize
where
    K::Item: IntoIterator,
{
    keeping.held()
}

pub fn main() {
    println!("{}", Wrapper(Countdown(8)).held());
    println!("{}", held_through(Wrapper(Countdown(7))));
}
