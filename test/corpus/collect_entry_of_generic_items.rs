// A std collection's `FromIterator` given for a callee's bound, where the
// items are a type only a caller knows, a type parameter or an iterator's
// `Item`: its `from_iter` collects as std's does, by the items' layout,
// which the caller is given, as are its own (#2442). Of a trait's default
// in a blanket impl, the impl's dictionary is given it; of callers that
// call each other, it's asked once of each. A set's or a map's keys' class
// and a B-tree's entries' order are its caller's too, and std's shunt of
// `Option`s drops what it collected at the first `None`.
use std::collections::{BTreeMap, BTreeSet, HashSet};

fn gather<I: Iterator, B: FromIterator<I::Item>>(i: I) -> B {
    i.collect()
}

fn of_param<T, I: Iterator<Item = T>>(i: I) -> Vec<T> {
    gather(i)
}

fn of_item<I: Iterator>(i: I) -> Vec<I::Item> {
    gather(i)
}

fn as_set<I: Iterator>(i: I) -> BTreeSet<I::Item>
where
    I::Item: Ord,
{
    gather(i)
}

fn as_hashed<I: Iterator>(i: I) -> HashSet<I::Item>
where
    I::Item: std::hash::Hash + Eq,
{
    gather(i)
}

fn as_map<K: Ord, V, I: Iterator<Item = (K, V)>>(i: I) -> BTreeMap<K, V> {
    gather(i)
}

fn all_some<T, I: Iterator<Item = Option<T>>>(i: I) -> Option<Vec<T>> {
    gather(i)
}

fn there<I: Iterator>(i: I, n: u32) -> Vec<I::Item> {
    if n == 0 { gather(i) } else { back(i, n - 1) }
}

fn back<I: Iterator>(i: I, n: u32) -> Vec<I::Item> {
    if n == 0 { of_item(i) } else { there(i, n - 1) }
}

trait Gathered: Iterator + Sized {
    fn gathered(self) -> Vec<Self::Item> {
        gather(self)
    }
}

impl<I: Iterator> Gathered for I {}

struct Loud(u32);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn main() {
    println!("{:?}", of_param(1..4u32));
    println!("{:?}", of_param(vec![String::from("a"), String::from("b")].into_iter()));
    println!("{:?}", of_item((1..4u8).map(|n| n * 2)));
    println!("{:?}", of_item(vec![(1u16, 'x')].into_iter()));
    println!("{:?}", as_set([3, 1, 2u64].into_iter()));
    let mut hashed: Vec<_> = as_hashed(["b", "a", "b"].into_iter()).into_iter().collect();
    hashed.sort();
    println!("{hashed:?}");
    println!("{:?}", as_map([(2u8, "two"), (1, "one"), (2, "again")].into_iter()));
    println!("{:?}", all_some([Some(1u32), Some(2)].into_iter()));
    println!(
        "{:?}",
        all_some([Some(String::from("x")), None, Some(String::from("y"))].into_iter())
    );
    println!("{:?}", there(1..3i32, 3));
    println!("{:?}", (1..4u32).gathered());
    println!("{:?}", vec![String::from("c")].into_iter().gathered());
    let louds = vec![Loud(1), Loud(2)].into_iter().filter(|l| l.0 > 1).gathered();
    println!("kept {}", louds.len());
    let louds = of_item(vec![Loud(3), Loud(4)].into_iter());
    println!("kept {}", louds.len());
    let louds = as_map(vec![(2u8, Loud(5)), (1, Loud(6)), (2, Loud(7))].into_iter());
    println!("kept {}", louds.len());
    let louds = all_some(vec![Some(Loud(8)), None, Some(Loud(9))].into_iter());
    println!("kept {}", louds.is_some());
}
