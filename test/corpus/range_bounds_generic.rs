// A range's bounds in generic code, `R: RangeBounds<usize>`, as arrayvec's
// `drain` takes them: the range's dictionary gives each, a `Bound` of what
// its field holds, of every range of std's and a pair of `Bound`s.
use std::ops::{Bound, RangeBounds};

fn start<R: RangeBounds<usize>>(r: R) -> usize {
    match r.start_bound() {
        Bound::Included(&i) => i,
        Bound::Excluded(&i) => i + 1,
        Bound::Unbounded => 0,
    }
}

fn end<R: RangeBounds<usize>>(r: R, len: usize) -> usize {
    match r.end_bound() {
        Bound::Included(&j) => j + 1,
        Bound::Excluded(&j) => j,
        Bound::Unbounded => len,
    }
}

fn shown(b: Bound<&usize>) -> String {
    match b {
        Bound::Included(i) => format!("in {i}"),
        Bound::Excluded(i) => format!("ex {i}"),
        Bound::Unbounded => "none".to_string(),
    }
}

fn main() {
    println!("{} {} {} {}", start(2..5), start(..5), end(2..=5, 9), end(3.., 9));
    println!("{} {} {}", end(.., 9), start(1..=1), end(..=4, 9));
    println!("{}", shown(Bound::Excluded(&7)));
    println!("{} {}", start((Bound::Excluded(1), Bound::Unbounded)), end((Bound::Unbounded, Bound::Included(3)), 9));
}
