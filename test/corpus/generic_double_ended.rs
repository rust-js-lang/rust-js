// An associated type a bound says is double-ended, `I::IntoIter:
// DoubleEndedIterator`, is a JS iterator that knows its back, as a type
// parameter's is; and a `Copied` of one is the one it copies, which knows
// its back and its length where it does (ADR 0480).
fn backwards<I: DoubleEndedIterator>(it: I) -> Vec<I::Item> {
    it.rev().collect()
}

fn last_even<I: DoubleEndedIterator<Item = u32>>(mut it: I) -> Option<u32> {
    it.rfind(|x| x % 2 == 0)
}

fn last_of<I: Iterator + DoubleEndedIterator>(mut it: I) -> Option<I::Item> {
    it.next_back()
}

fn rev_all<I>(iterable: I) -> std::iter::Rev<I::IntoIter>
where
    I: IntoIterator,
    I::IntoIter: DoubleEndedIterator,
{
    iterable.into_iter().rev()
}

fn main() {
    println!("{:?}", backwards(vec![1, 2, 3].into_iter()));
    println!("{:?}", last_even([1u32, 2, 3, 4, 5].iter().copied()));
    println!("{:?}", rev_all(vec!['a', 'b']).collect::<String>());
    println!("{:?}", last_of(vec![7, 8].into_iter()));
}
