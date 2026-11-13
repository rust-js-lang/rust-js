// The consumer of that library, whose call asks of a key nested past what
// the library's contract can say: refused, not a crash (#1713).
use dep::*;
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::hash::{Hash, Hasher};
pub fn main() {
    println!("{}", vec!['a', 'b', 'a'].into_iter().positions_seen());
    println!("{}", vec![Digit(1), Digit(11), Digit(2)].into_iter().positions_seen());
    let ordered = vec![3u32, 1, 2].into_iter().ordered();
    println!("{:?}", ordered.keys().collect::<Vec<_>>());
    let backwards = vec![Backwards(1), Backwards(3), Backwards(2)].into_iter().ordered();
    println!("{:?}", backwards.keys().collect::<Vec<_>>());
    println!("{}", distinct(&[1u32, 2, 1]));
    println!("{}", distinct(&[Digit(4), Digit(14), Digit(5)]));
    println!("{}", pairs(&["a", "b", "a", "b"]));
    println!("{:?}", sorted(&[Backwards(5), Backwards(7), Backwards(5)]));
    println!("{:?}", sorted(&["b", "a"]));
}
