// What's given behind an `impl` or a bound and stepped from its back, as
// Rust steps it: a `chain`, a `copied()` and a `map` returned as an `impl
// DoubleEndedIterator`, an `I::IntoIter` a bound says is double-ended, a
// held `rev()` of a mapped range, and `by_ref()` of a held adapter (#382).
fn chained(v: &[u32]) -> impl DoubleEndedIterator<Item = &u32> + '_ {
    v.iter().chain(v.iter().take(1))
}
fn copied(v: &[u32]) -> impl DoubleEndedIterator<Item = u32> + ExactSizeIterator + '_ {
    v.iter().copied()
}
fn mapped(v: &[u32]) -> impl DoubleEndedIterator<Item = u32> + ExactSizeIterator + '_ {
    v.iter().map(|x| x * 2)
}
fn last_of<C>(c: C) -> Option<C::Item>
where
    C: IntoIterator,
    C::IntoIter: DoubleEndedIterator,
{
    c.into_iter().rev().next()
}
fn tens(x: u32) -> u32 {
    x * 10
}
fn main() {
    let v = vec![1, 2, 3];
    println!("{:?}", chained(&v).rev().collect::<Vec<_>>());
    let c = copied(&v);
    println!("{} {:?}", c.len(), c.rev().collect::<Vec<_>>());
    let mut m = mapped(&v);
    println!("{:?} {} {:?}", m.next_back(), m.len(), m.rev().next());
    println!("{:?} {:?}", last_of(vec![4, 5]), last_of(&v));
    let mut held = (1..5).map(tens).rev();
    println!("{:?} {:?} {}", held.next(), held.next_back(), held.len());
    let mut it = v.iter().map(|x| x + 1);
    let back = it.by_ref().rev().next();
    println!("{back:?} {:?} {:?}", it.next(), it.next_back());
    let r = v.iter().rev();
    println!("{} {:?}", r.len(), r.rev().next());
}
