// A `&mut Box<I>` of an iterator `I` is what a `&mut I` is, the iterator
// itself, as a `Box` is what it holds in JS (the user's decision, #1314,
// amending #1222): `by_ref()` of a boxed iterator, which threw as it lent
// the box's cell to what steps it, a `&mut boxed` given to generic code, a
// `Box<dyn Iterator>`'s, and a write of the whole through one, `*b =
// Box::new(..)` and `mem::replace`, of a `Vec`'s iterator, a range and a
// `dyn`, and of what the box holds, `**b = ..`, `mem::replace(&mut **b,
// ..)`, a reborrow of it by its deref and a step through `(**b)`.
fn total<I: Iterator<Item = u32>>(it: &mut I) -> u32 {
    it.take(2).sum()
}

fn reset(b: &mut Box<std::vec::IntoIter<u32>>) {
    **b = vec![9, 8].into_iter();
}

fn reset_range(b: &mut Box<std::ops::Range<u32>>) {
    **b = 20..22;
}

fn reset_dyn(b: &mut Box<dyn Iterator<Item = u32>>) {
    *b = Box::new(30..32u32);
}

fn swap_in(b: &mut Box<std::ops::Range<u32>>) -> Box<std::ops::Range<u32>> {
    std::mem::replace(b, Box::new(40..41))
}

fn replaced(b: &mut Box<std::ops::Range<u32>>) -> std::ops::Range<u32> {
    std::mem::replace(&mut **b, 50..52)
}

fn reborrowed(b: &mut Box<std::ops::Range<u32>>) -> Option<u32> {
    let r: &mut std::ops::Range<u32> = b;
    r.next()
}

fn through(b: &mut Box<std::vec::IntoIter<u32>>) -> Option<u32> {
    (**b).next_back()
}

fn main() {
    let mut boxed = Box::new(vec![1u32, 2, 3, 4, 5].into_iter());
    let first: Vec<u32> = boxed.by_ref().take(2).collect();
    println!("{first:?} {:?}", boxed.next());
    println!("{}", total(&mut boxed));
    println!("{:?}", boxed.collect::<Vec<_>>());
    let mut dynamic: Box<dyn Iterator<Item = u32>> = Box::new(1..6u32);
    let two: Vec<u32> = dynamic.by_ref().take(2).collect();
    println!("{two:?} {:?}", dynamic.next());
    reset_dyn(&mut dynamic);
    println!("{:?}", dynamic.collect::<Vec<_>>());
    let mut again = Box::new(vec![1u32, 2, 3].into_iter());
    again.next();
    reset(&mut again);
    println!("{:?}", again.collect::<Vec<_>>());
    let mut ranged = Box::new(0..5u32);
    ranged.next();
    reset_range(&mut ranged);
    println!("{:?}", ranged.collect::<Vec<_>>());
    let mut held = Box::new(0..3u32);
    held.next();
    let old = swap_in(&mut held);
    println!("{:?} {:?}", old.collect::<Vec<_>>(), held.collect::<Vec<_>>());
    let mut swapped = Box::new(0..3u32);
    let before = replaced(&mut swapped);
    println!("{before:?} {swapped:?}");
    let mut lent = Box::new(5..9u32);
    println!("{:?} {lent:?}", reborrowed(&mut lent));
    let mut items = Box::new(vec![1u32, 2, 3].into_iter());
    println!("{:?} {:?}", through(&mut items), items.collect::<Vec<_>>());
}
