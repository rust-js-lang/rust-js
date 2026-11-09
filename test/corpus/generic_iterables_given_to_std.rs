//@ library-differs
// Generic code given an iterator as it is, made by a function it's given,
// `F: Fn() -> I`, hands it to std's functions that take an iterable, an
// `extend` of each collection and a `Debug` builder's `entries`: each steps
// it by its `Iterator`, the crate's own iterator as it is (#1147).
use std::collections::{BTreeMap, BTreeSet, HashSet, LinkedList, VecDeque};
use std::fmt;

/// The crate's own iterator, which prints its steps.
struct It<I>(I);

impl<I: Iterator> Iterator for It<I> {
    type Item = I::Item;

    fn next(&mut self) -> Option<I::Item> {
        println!("next");
        self.0.next()
    }
}

fn extended<F: Fn() -> I, I: Iterator<Item = u32>>(make: F) {
    let mut v = vec![0];
    v.extend(make());
    let mut d = VecDeque::from([0]);
    d.extend(make());
    let mut l = LinkedList::from([0]);
    l.extend(make());
    let mut s = BTreeSet::from([0]);
    s.extend(make());
    let mut h = HashSet::from([0]);
    h.extend(make());
    let mut h: Vec<u32> = h.into_iter().collect();
    h.sort();
    println!("{v:?} {d:?} {l:?} {s:?} {h:?}");
}

fn keyed<F: Fn() -> I, I: Iterator<Item = (u32, char)>>(make: F) {
    let mut m = BTreeMap::from([(0, 'z')]);
    m.extend(make());
    println!("{m:?}");
}

fn texts<F: Fn() -> I, I: Iterator<Item = char>>(make: F) {
    let mut s = String::from(">");
    s.extend(make());
    println!("{s}");
}

struct Listed<F>(F);

impl<F, I> fmt::Debug for Listed<F>
where
    F: Fn() -> I,
    I: Iterator,
    I::Item: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_list().entries((self.0)()).finish()
    }
}

fn main() {
    extended(|| It(vec![1, 2].into_iter()));
    extended(|| vec![3u32].into_iter());
    keyed(|| It(vec![(1, 'a'), (2, 'b')].into_iter()));
    texts(|| It("ab".chars()));
    let shown = format!("{:?}", Listed(|| It(vec![1u32, 2].into_iter())));
    println!("{shown}");
}
