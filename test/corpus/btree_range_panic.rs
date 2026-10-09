//@ run-fail: range start is greater than range end in BTreeMap
// A B-tree's range whose start is past its end panics with std's message
// (ADR 0325).

use std::collections::BTreeMap;

fn main() {
    let tree = BTreeMap::from([(1, "a"), (2, "b")]);
    let (start, end) = (5, 2);
    println!("{:?}", tree.range(start..end).count());
}
