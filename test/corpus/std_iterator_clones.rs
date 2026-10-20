// std's iterators of an array, or of what's made one, clone where they are:
// a string's `chars()` and `char_indices()`, an `Option`'s and a
// `Result`'s `iter()`, a `VecDeque`'s and a `LinkedList`'s, and a `rev()`
// of one, stepped from either end; a `rev()` of generic code's iterator
// too (ADR 0530).
use std::collections::{LinkedList, VecDeque};

fn reversed_twice<I: DoubleEndedIterator<Item = u32> + Clone>(it: I) -> (Vec<u32>, Vec<u32>) {
    let mut back = it.rev();
    back.next();
    (back.clone().collect(), back.collect())
}

fn back_len<I: DoubleEndedIterator<Item = u32> + ExactSizeIterator>(it: I) -> usize {
    let mut back = it.rev();
    back.next();
    back.len()
}

fn main() {
    let text = "héllo";
    let mut chars = text.chars();
    chars.next();
    let later = chars.clone();
    println!("{:?} {:?}", chars.collect::<String>(), later.rev().collect::<String>());

    let mut indices = text.char_indices();
    indices.next();
    let copy = indices.clone();
    println!("{:?} {:?}", indices.next_back(), copy.collect::<Vec<_>>());

    let some = Some(3);
    let result: Result<u32, String> = Ok(4);
    let (mut a, mut b) = (some.iter(), result.iter());
    let (a2, b2) = (a.clone(), b.clone());
    println!("{:?} {:?} {:?} {:?}", a.next(), b.next(), a2.count(), b2.count());

    let deque: VecDeque<u32> = (1..5).collect();
    let mut walk = deque.iter();
    walk.next();
    let rest = walk.clone();
    let list: LinkedList<u32> = (5..9).collect();
    let mut steps = list.iter();
    steps.next_back();
    let from_here = steps.clone();
    println!("{:?} {:?} {:?} {:?}", walk.len(), rest.rev().collect::<Vec<_>>(), steps.next(), from_here.collect::<Vec<_>>());

    let mut back = vec![10, 20, 30, 40].into_iter().rev();
    back.next();
    println!("{:?} {:?} {:?}", back.next_back(), back.clone().collect::<Vec<_>>(), back.len());
    println!("{:?} {}", reversed_twice(vec![1, 2, 3].into_iter()), back_len(vec![4, 5, 6].into_iter()));

    let mut rows = vec![vec![1], vec![2]].into_iter().rev();
    let mut copied = rows.clone();
    copied.next().unwrap().push(9);
    println!("{:?} {:?}", rows.next(), copied.next());
}
