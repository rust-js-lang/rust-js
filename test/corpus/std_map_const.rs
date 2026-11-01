// A constant of std's `BTreeMap` is a new map at each use: one grown, by
// `extend` or by a function given it, leaves the next as the constant says.
// Its JS value is a copy of one map where it's read, as std's fields are no
// JS value's to say it can't change.
use std::collections::BTreeMap;

const SCORES: BTreeMap<&str, u32> = BTreeMap::new();

fn with_ada(mut scores: BTreeMap<&'static str, u32>) -> BTreeMap<&'static str, u32> {
    scores.insert("ada", 3);
    scores
}

fn main() {
    let given = with_ada(SCORES);
    let mut grown = SCORES;
    grown.extend([("bo", 1), ("cy", 2)]);
    let fresh = SCORES;
    println!("{given:?} {grown:?} {fresh:?} {}", SCORES.len());
}
