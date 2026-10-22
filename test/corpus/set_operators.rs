// A set's operators, `&a - &b`, `|`, `&` and `^`: a set of their algebra's
// items, a B-tree's in its order.
use std::collections::{BTreeSet, HashSet};

fn main() {
    let a: BTreeSet<i32> = [1, 2, 3].into_iter().collect();
    let b: BTreeSet<i32> = [2, 3, 4].into_iter().collect();
    println!("{:?} {:?} {:?} {:?}", &a - &b, &a | &b, &a & &b, &a ^ &b);
    let only = &a - &b;
    println!("{} {} {}", only.contains(&1), only.contains(&2), only.len());

    let names: BTreeSet<String> = ["ann", "bo"].map(String::from).into();
    let more: BTreeSet<String> = ["bo", "cy"].map(String::from).into();
    println!("{:?}", &names | &more);

    let pairs: HashSet<(i32, char)> = [(1, 'a'), (2, 'b')].into();
    let others: HashSet<(i32, char)> = [(2, 'b'), (3, 'c')].into();
    let mut both: Vec<_> = (&pairs & &others).into_iter().collect();
    let mut either: Vec<_> = (&pairs ^ &others).into_iter().collect();
    both.sort();
    either.sort();
    println!("{both:?} {either:?}");
}
