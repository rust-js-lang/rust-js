// A `String` extended by what's given as an `IntoIterator` of `char`s or
// strings, each stepped as Rust steps it: a range's `char`s, an `Option`'s
// one or none, a `Result`'s `Ok`, a B-tree's in order (ADR 0445).
use std::collections::BTreeSet;

fn main() {
    let mut text = String::new();
    text.extend('a'..='e');
    text.extend('m'..'p');
    text.extend(Some('x'));
    text.extend(None::<char>);
    text.extend(Ok::<char, ()>('k'));
    text.extend(BTreeSet::from(['z', 'y']));
    text.extend(vec!["ab", "cd"]);
    println!("{text}");
}
