// What a generic function bounded by `IntoIterator` is given, the JS
// iterator of its items (ADR 0061), `iterable_items`': a `&mut` collection's
// handles, as `for x in &mut v` makes, an `Option`'s, a B-tree's in its
// keys' order; and a `&mut` range, stepped where it is, `extend`'s too.

use std::collections::{BTreeMap, HashSet, VecDeque};
fn total<'a, I: IntoIterator<Item = &'a u32>>(i: I) -> u32 {
    i.into_iter().sum()
}
fn bump<'a, I: IntoIterator<Item = &'a mut u32>>(i: I) {
    for x in i {
        *x += 1;
    }
}
fn count<I: IntoIterator>(i: I) -> usize {
    i.into_iter().count()
}
fn firsts<I>(i: I) -> Vec<String>
where
    I: IntoIterator,
    I::Item: std::fmt::Debug,
{
    i.into_iter().take(2).map(|x| format!("{x:?}")).collect()
}
fn main() {
    let mut v = vec![1u32, 2, 3];
    let arr = [4u32, 5];
    println!(
        "{} {} {} {}",
        total(&v),
        total(&arr),
        total(&v[1..]),
        total(v.iter().rev())
    );
    bump(&mut v);
    bump(v.iter_mut().skip(1));
    let mut a2 = [10u32, 20];
    bump(&mut a2);
    println!("{v:?} {a2:?}");
    let o = Some(7u32);
    println!("{} {} {}", total(&o), count(o), count(None::<u8>));
    let m: BTreeMap<&str, u32> = [("a", 1), ("b", 2)].into_iter().collect();
    println!("{} {:?} {:?}", total(m.values()), firsts(&m), firsts(m.keys()));
    let s: HashSet<u8> = [3, 3, 4].into_iter().collect();
    println!("{} {}", count(&s), count(s));
    let mut d: VecDeque<u32> = (1..4).collect();
    d.push_front(0);
    bump(&mut d);
    println!("{} {:?}", total(&d), firsts(d.iter().rev()));
    println!("{:?} {:?}", firsts("héy".chars()), firsts(vec![vec![1], vec![]]));
    let mut it = v.iter();
    it.next();
    println!("{} {}", total(&mut it), count(it));
    let mut unsorted: BTreeMap<u8, char> = BTreeMap::new();
    unsorted.insert(9, 'z');
    unsorted.insert(1, 'a');
    println!("{:?}", firsts(&unsorted));
    let mut r = 1..6u32;
    let mut out = vec![0u32];
    out.extend(&mut r);
    println!("{out:?} {:?}", r.next());
}
