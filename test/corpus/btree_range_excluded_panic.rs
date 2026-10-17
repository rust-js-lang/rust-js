//@ run-fail: range start and end are equal and excluded in BTreeMap
// A B-tree's range whose start and end are equal and both excluded panics
// with std's message (ADR 0325).

use std::collections::BTreeMap;
use std::ops::Bound::Excluded;

fn main() {
    let tree = BTreeMap::from([(1, "a"), (2, "b")]);
    let at = 2;
    println!("{:?}", tree.range((Excluded(&at), Excluded(&at))).count());
}
