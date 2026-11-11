//! A consumer that compiles the library's impl itself, given a `&mut` of
//! an array to `partition` and calling `collect_array`, and calls `last_of`,
//! which asks its items' size (#1409).
use dep::Tools;

/// A caller's own iterator given to the impl's defaults, whose facts it
/// asks of its own callers.
fn ends<I: DoubleEndedIterator<Item = i32> + Clone>(it: I) -> (Vec<i32>, i32) {
    (it.clone().last_of(1), it.dot(vec![1, 10, 100]))
}

pub fn main() {
    let mut data = [7, 1, 1, 9, 1, 1, 3];
    let i = dep::partition(&mut data, |elt| *elt >= 3);
    println!("{i} {data:?}");
    let v = [1, 2];
    println!("{:?}", v.iter().cloned().collect_array::<2>());
    println!("{:?}", v.iter().cloned().last_of(1));
    println!("{:?}", v.iter().cloned().dot(vec![3, 4]));
    println!("{:?}", ends([5, 6, 7].iter().cloned()));
    println!("{:?}", ends(1..4));
    let w = [1, 2, 3];
    println!("{:?}", w.iter().cloned().collect_array::<2>());
}
