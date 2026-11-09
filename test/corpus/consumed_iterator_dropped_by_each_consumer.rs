// An iterator with a destructor that std's code consumes is dropped where
// that code ends, before what it made is read, by every collection's
// `from_iter`, which `collect()` calls, and `collect()` itself, a generic
// `C: FromIterator`'s, `sum`, `product`, `max`, `count` and `last`: one path
// drops what's consumed, which `from_iter` once had a second of, that never
// dropped it (#1116, ADR 0160).
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet, VecDeque};

struct Loud<I>(I, &'static str);

impl<I: Iterator> Iterator for Loud<I> {
    type Item = I::Item;
    fn next(&mut self) -> Option<I::Item> {
        self.0.next()
    }
}

impl<I> Drop for Loud<I> {
    fn drop(&mut self) {
        println!("drop {}", self.1);
    }
}

fn loud<T>(v: Vec<T>, name: &'static str) -> Loud<std::vec::IntoIter<T>> {
    Loud(v.into_iter(), name)
}

fn built<C: FromIterator<u32>>(it: Loud<std::vec::IntoIter<u32>>) -> C {
    C::from_iter(it)
}

fn main() {
    let m = HashMap::<u32, u32>::from_iter(loud(vec![(1, 2)], "hashmap"));
    println!("{}", m.len());
    let b = BTreeMap::from_iter(loud(vec![(1, 2)], "btreemap"));
    println!("{b:?}");
    let s = HashSet::<u32>::from_iter(loud(vec![1, 1], "hashset"));
    println!("{}", s.len());
    let t = BTreeSet::from_iter(loud(vec![2, 1], "btreeset"));
    println!("{t:?}");
    let text = String::from_iter(loud(vec!['h', 'i'], "string"));
    println!("{text}");
    let d = VecDeque::from_iter(loud(vec![1, 2], "deque"));
    println!("{d:?}");
    let bx = Box::<[u32]>::from_iter(loud(vec![3], "box"));
    println!("{bx:?}");
    let c: BTreeSet<u32> = loud(vec![5], "collected").collect();
    println!("{c:?}");
    let g: Vec<u32> = built(loud(vec![7], "generic vec"));
    println!("{g:?}");
    let g: BTreeSet<u32> = built(loud(vec![8], "generic set"));
    println!("{g:?}");
    println!("{}", loud(vec![1u32, 2], "sum").sum::<u32>());
    println!("{}", loud(vec![2u32, 3], "product").product::<u32>());
    println!("{:?}", loud(vec![4u32], "max").max());
    println!("{}", loud(vec![4u32], "count").count());
    println!("{:?}", loud(vec![4u32, 5], "last").last());
}
