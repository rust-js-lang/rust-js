// A clone of std's compiled adapter its use gave its back, `$knowing`'s:
// the clone, std's compiled `Clone`, made as the one it clones is, knows
// what that one knows, so a `rev()` over a clone of a `map` steps it from
// its back (#1976). Generic code given one beside it, `show`, makes the
// chain std's struct, which a clone made as its type says lost; so does
// one handed through generic functions, `through`. Each clone is stepped
// apart from what it cloned: from its back, by a `rev()`'s `next`, `nth`
// and `fold`, the original as it was; and the original stepped, the clone
// as it was.
use std::fmt::Debug;

fn show<I: Iterator + Clone>(it: I)
where
    I::Item: Debug,
{
    let mut copy = it.clone();
    copy.next();
    println!("show {:?} {:?}", copy.collect::<Vec<_>>(), it.collect::<Vec<_>>());
}

fn through<I: DoubleEndedIterator + ExactSizeIterator + Clone>(
    it: I,
) -> impl DoubleEndedIterator<Item = I::Item> + ExactSizeIterator + Clone {
    it
}

fn rev(items: &[i32]) {
    let e = items.iter().map(|x| x.min(&9)).copied().rev();
    let mut e2 = e.clone();
    e2.next();
    println!("rev {:?} {:?}", e2.collect::<Vec<_>>(), e.collect::<Vec<_>>());
    show(items.iter().map(|x| x.min(&9)).copied().map(|x| x * 10));
}

fn clone_stepped(items: &[i32]) {
    let m = items.iter().map(|x| x.min(&9)).copied().rev();
    let mut m2 = m.clone();
    println!("clone's back {:?} {:?}", m2.next(), m2.nth(1));
    println!("clone's front {:?}", m2.next_back());
    println!("clone's rfold {}", m2.fold(0, |sum, x| sum * 100 + x));
    println!("original {:?}", m.collect::<Vec<_>>());
    show(items.iter().map(|x| x.min(&9)).copied().rev());
}

fn original_stepped(items: &[i32]) {
    let mut n = items.iter().map(|x| x.min(&9)).copied().rev();
    let n2 = n.clone();
    println!("original's back {:?} {:?}", n.next(), n.nth(1));
    println!("original's rfold {}", n.fold(0, |sum, x| sum * 100 + x));
    println!("clone {:?}", n2.collect::<Vec<_>>());
    show(items.iter().map(|x| x.min(&9)).copied());
}

fn handed_through(items: &[i32]) {
    let o = through(through(items.iter()).map(|x| x.min(&9))).copied().rev();
    let mut o2 = through(o).clone();
    o2.next();
    println!("through {:?}", o2.collect::<Vec<_>>());
}

fn main() {
    let items = [1, 2, 3, 4, 5];
    rev(&items);
    clone_stepped(&items);
    original_stepped(&items);
    handed_through(&items);
}
