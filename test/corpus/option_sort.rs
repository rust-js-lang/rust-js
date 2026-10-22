// `None` is less than any `Some`, as `Option`'s `Ord` says: sorting puts
// it first, by `sort`, `sort_by` and `sort_by_key`, and a `BTreeSet`
// lists it first. JS's own `sort` would put it last.
use std::collections::BTreeSet;

fn main() {
    let mut scores = vec![Some(2), None, Some(-1), None];
    scores.sort();
    println!("{scores:?}");
    let mut names = vec![Some("b".to_string()), None, Some("a".to_string())];
    names.sort_by(|a, b| b.cmp(a));
    println!("{names:?}");
    let mut words = vec![Some("pear"), None, Some("fig")];
    words.sort_by_key(|w| w.map(str::len));
    println!("{words:?}");
    let seen: BTreeSet<Option<u8>> = [Some(3), None, Some(1)].into_iter().collect();
    println!("{seen:?}");
}
