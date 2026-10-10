// `next_back()` of a std iterator: its last item, which a kept one then
// hasn't, as `next()` takes its first.
use std::collections::BTreeMap;

fn hint<I: Iterator>(it: &I) -> (usize, Option<usize>) {
    it.size_hint()
}

fn main() {
    let v = vec![1, 2, 3, 4, 5];
    let mut it = v.iter();
    println!("{:?} {:?} {:?}", it.next(), it.next_back(), it.next_back());
    println!("{} {:?} {:?} {:?}", it.len(), it.size_hint(), hint(&it), it.as_slice());
    let rest: Vec<_> = it.clone().collect();
    println!("{rest:?}");
    let mut taken = v.iter();
    taken.next_back();
    let front: Vec<_> = taken.collect();
    println!("{front:?}");
    for x in it.by_ref() {
        print!("{x} ");
    }
    println!("{:?} {:?}", it.next(), it.next_back());

    let mut both = (1..=4).collect::<Vec<i32>>().into_iter();
    while let (Some(a), Some(b)) = (both.next(), both.next_back()) {
        print!("({a}, {b}) ");
    }
    println!("{:?}", both.next());

    let tree: BTreeMap<u8, &str> = [(1, "a"), (2, "b"), (3, "c")].into();
    println!("{:?} {:?}", tree.range(..3).next_back(), tree.iter().next_back());
    println!("{:?}", v.iter().map(|x| x * 10).next_back());
    println!("{:?} {:?}", "héllo".chars().next_back(), "a,b,c".split(',').next_back());

    let maybe: Vec<Option<i32>> = vec![Some(1), None];
    let mut options = maybe.iter();
    println!("{:?} {:?} {:?}", options.next_back(), options.next_back(), options.next_back());
    let mut peeked = maybe.iter().peekable();
    peeked.next_back();
    peeked.next_back();
    println!("{:?} {:?}", peeked.peek(), (0..=3).next_back());

    let mut peeking = v.iter().peekable();
    let first = peeking.peek().copied();
    println!("{first:?} {:?}", peeking.next_back());
    while peeking.next_back().is_some() {}
    let left = peeking.peek().copied();
    println!("{left:?} {:?}", peeking.next_if(|_| true));

    let mut chars = "xyz".chars();
    chars.next_back();
    println!("{:?}", chars.as_str());
}
