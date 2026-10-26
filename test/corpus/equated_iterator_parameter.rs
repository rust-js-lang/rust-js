// A type parameter the where-clauses make an associated type, `U` of
// `I: Iterator<Item: IntoIterator<IntoIter = U>>`, is bound as that type
// is, an iterator (ADR 0924): a `&mut U` is the iterator, given to a
// closure as it is, as std's `Flatten` holds its inner one.

fn front<I, U>(outer: &mut I, slot: &mut Option<U>, mut f: impl FnMut(&mut U) -> Option<i32>) -> Option<i32>
where
    I: Iterator<Item: IntoIterator<IntoIter = U>>,
{
    let inner = slot.insert(outer.next()?.into_iter());
    f(inner)
}

fn main() {
    let mut outer = vec![vec![1, 2], vec![3]].into_iter();
    let mut slot = None;
    let a = front(&mut outer, &mut slot, |inner| inner.next());
    let b = front(&mut outer, &mut slot, |inner| inner.next());
    let c = front(&mut outer, &mut slot, |inner| inner.next());
    println!("{a:?} {b:?} {c:?} {:?}", slot.map(|inner| inner.len()));
}
