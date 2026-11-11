//! A consumer calling the library's `partition` of a `&mut` of an array and
//! its default `collect_array` of a `cloned()`, as itertools' test_core.rs's
//! `part` and `collect_array` do, which together compile the library's
//! defaults here, generic over the impl's `T` (ADR 0914, #1409).
use dep::Tools;

pub fn main() {
    let mut data = [7, 1, 1, 9, 1, 1, 3];
    let i = dep::partition(&mut data, |elt| *elt >= 3);
    println!("{i} {data:?}");
    let v = [1, 2];
    println!("{:?}", v.iter().cloned().collect_array::<2>());
    let w = [1, 2, 3];
    println!("{:?}", w.iter().cloned().collect_array::<2>());
}
