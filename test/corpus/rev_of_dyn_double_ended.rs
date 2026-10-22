// `rev()` of a `dyn DoubleEndedIterator`, a JS iterator that knows its back,
// `$rev(it)` (ADR 0429), given away and held, after a `next_back()`.
fn backwards(it: Box<dyn DoubleEndedIterator<Item = u32>>) -> Vec<u32> {
    it.rev().collect()
}
fn main() {
    println!("{:?}", backwards(Box::new(vec![1, 2, 3].into_iter())));
    let mut both: Box<dyn DoubleEndedIterator<Item = u32>> = Box::new(1..5);
    println!("{:?} {:?}", both.next_back(), both.rev().collect::<Vec<_>>());
}
