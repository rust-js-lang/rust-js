// A generic function's `Map<I, F>` stepped from its back, beside another
// function's `Map` of a slice's items that's cloned: the generic one can't
// be that one, its bounds say, its items `u32`s and not `&i32`s, so it's made
// as one never cloned (#955); and so a generic `Chain<I, I>` beside a cloned
// `Chain` of a slice's items (#1002). A generic one whose bounds let it be
// the one cloned, made in generic code and cloned by its caller, is cloned
// as Rust clones it, from where it is.
use std::iter::Map;

fn tripled_back<I: DoubleEndedIterator<Item = u32>, F: FnMut(u32) -> u32>(items: I, f: F) -> Vec<u32> {
    items.map(f).rfold(Vec::new(), |mut all, x| {
        all.push(x * 3);
        all
    })
}

fn chained_back<I: DoubleEndedIterator<Item = u32>>(a: I, b: I) -> Option<u32> {
    a.chain(b).map(|x| x + 1).next_back()
}

fn mapped<'a, I: Iterator<Item = &'a i32>, F: FnMut(&'a i32) -> i32>(items: I, f: F) -> Map<I, F> {
    items.map(f)
}

fn main() {
    let a = [1, 2, 3];
    let mut m = mapped(a.iter(), |x| x * 10);
    m.next();
    let m2 = m.clone();
    println!("{:?} {:?}", m.collect::<Vec<_>>(), m2.rev().collect::<Vec<_>>());
    println!("{:?}", tripled_back(vec![1, 2, 3].into_iter(), |x| x + 1));
    println!("{:?}", tripled_back(5..7, |x| x * 2));
    let mut c = a.iter().chain(a.iter());
    c.next();
    let c2 = c.clone();
    println!("{:?} {:?}", c.count(), c2.rev().collect::<Vec<_>>());
    println!("{:?}", chained_back(vec![1, 2].into_iter(), vec![3, 4].into_iter()));
    println!("{:?}", chained_back(1..2, 5..5));
}
