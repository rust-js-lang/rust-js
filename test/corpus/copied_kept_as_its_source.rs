// A `copied()` is the iterator it copies, as rust-js makes it, never std's
// `Copied` struct: a copy of a `Copy` item is the item (ADR 0480), so the
// hand form is the meaning and stays (ADR 1136, kept, proved equal). Here,
// with what the copy matrix's `copied` rows lack: items that are a `Copy`
// struct, an object in JS, where a copy missed would alias, each copy
// changed and the array's item read after; and a `copied()` a function
// returns, held, lent, given as a `dyn`, stepped from its back and asked
// its length.
#[derive(Clone, Copy, Debug)]
struct P(u32, u32);

fn points() -> [P; 4] {
    [P(1, 10), P(2, 20), P(3, 30), P(4, 40)]
}

fn copies(ps: &[P]) -> std::iter::Copied<std::slice::Iter<'_, P>> {
    ps.iter().copied()
}

fn last_of<I: DoubleEndedIterator>(mut it: I) -> Option<I::Item> {
    it.next_back()
}

fn pass<I>(it: I) -> I {
    it
}

fn dup<I: Clone>(it: &I) -> I {
    it.clone()
}

fn main() {
    let ps = points();
    let mut it = ps.iter().copied();
    let mut first = it.next().unwrap();
    first.0 += 100;
    let mut back = it.next_back().unwrap();
    back.1 += 100;
    println!("{first:?} {back:?} {:?} {:?} len {}", ps[0], ps[3], it.len());

    let mut a = copies(&ps);
    let mut b = dup(&a);
    let mut x = a.next().unwrap();
    x.1 = 0;
    println!("{x:?} {:?} {:?}", b.next(), ps[0]);
    let mut c = pass(a.by_ref());
    println!("{:?} {:?} len {} {}", c.next_back(), a.next(), a.len(), b.len());

    let d: &mut dyn DoubleEndedIterator<Item = P> = &mut b;
    let mut y = d.next_back().unwrap();
    y.0 = 7;
    println!("{y:?} {:?} {:?}", d.next(), ps[3]);
    println!("{:?} {:?}", last_of(copies(&ps)), ps.iter().copied().rev().map(|p| p.0).collect::<Vec<_>>());

    let mut kept: Vec<P> = copies(&ps).collect();
    kept[1].0 = 99;
    println!("{:?} {:?}", kept[1], ps[1]);
}
