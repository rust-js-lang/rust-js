// `B::from_iter(it)` in generic code is the `collect()` it is, which
// std's `SpecFromIter` collects as the `Vec` it makes grows (#1239): of
// what its callers give, an iterator std collects by `next`, its first
// item, then room for its hint's lower bound and one more. What a caller
// gives it is made knowing its hint where JS's own adapters would lose it,
// a set's `iter().take(n)` and `skip(n)`, through a bound of `IntoIterator`
// as of `Iterator`, its `IntoIter` the iterator given (#2463).
use std::collections::{BTreeSet, HashSet, VecDeque};

fn vec_of<I: IntoIterator<Item = u32>>(it: I) -> Vec<u32> {
    Vec::from_iter(it)
}

fn deque_of<I: IntoIterator<Item = u32>>(it: I) -> VecDeque<u32> {
    VecDeque::from_iter(it)
}

fn by_iterator<I: Iterator<Item = u32>>(it: I) -> Vec<u32> {
    Vec::from_iter(it)
}

fn gathered<B: FromIterator<u32>, I: IntoIterator<Item = u32>>(it: I) -> B {
    B::from_iter(it)
}

fn main() {
    let set: HashSet<u32> = (0..9).collect();
    let sorted: BTreeSet<u32> = (0..40).collect();
    let v = vec_of(sorted.iter().take(4).chain(set.iter().take(3)).copied());
    println!("vec {} {}", v.len(), v.capacity());
    let d = deque_of(sorted.iter().take(4).chain(set.iter().take(3)).copied());
    println!("deque {} {}", d.len(), d.capacity());
    let skipped = vec_of(set.iter().skip(3).copied());
    println!("skipped {} {}", skipped.len(), skipped.capacity());
    let filtered = vec_of(set.iter().filter(|x| **x > 2).copied());
    println!("filtered {} {}", filtered.len(), filtered.capacity());
    let iterator = by_iterator(set.iter().take(3).chain(set.iter().take(2)).copied());
    println!("iterator {} {}", iterator.len(), iterator.capacity());
    let generic: Vec<u32> = gathered(sorted.iter().take(4).chain(set.iter().take(3)).copied());
    println!("generic {} {}", generic.len(), generic.capacity());
    let owned = vec_of(vec![1, 2, 3, 4, 5]);
    println!("owned {} {}", owned.len(), owned.capacity());
    let ranged = vec_of(0..6);
    println!("ranged {} {}", ranged.len(), ranged.capacity());
}
