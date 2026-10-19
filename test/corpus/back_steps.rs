// `nth_back(n)`, `rfold(init, f)` and `rfind(p)` of a double-ended
// iterator, a generic one's too, stepped from its back as `next_back`
// steps it (ADRs 0429, 0455): of items that could look like `None`, their
// `Some`. A map of one, a `by_ref()` too, is stepped from either end, its
// function called from the back as Rust's `Map` calls it. Of items with a
// destructor, `back_steps_drop`.
fn digits_back<I: DoubleEndedIterator<Item = u32>>(it: I) -> u32 {
    it.rfold(0, |a, b| a * 10 + b)
}

fn last_even<I: DoubleEndedIterator<Item = u32>>(mut it: I) -> Option<u32> {
    it.rfind(|n| n % 2 == 0)
}

fn second_last<I: DoubleEndedIterator>(mut it: I) -> Option<I::Item> {
    it.nth_back(1)
}

fn last_some<I: DoubleEndedIterator<Item = Option<u32>>>(mut it: I) -> Option<Option<u32>> {
    it.rfind(|item| item.is_none_or(|n| n > 1))
}

fn tagged<I: DoubleEndedIterator<Item = u32>>(it: I) -> u32 {
    it.map(|n| n + 100).rfold(0, |a, b| a * 1000 + b)
}

fn last_big<I: DoubleEndedIterator<Item = u32>>(mut it: I) -> (Option<u32>, Option<u32>) {
    let found = it.by_ref().map(|n| n * 2).rfind(|n| *n > 2);
    (found, it.next())
}

fn first_some<I: Iterator<Item = u32>, B>(mut it: I, f: impl FnMut(u32) -> Option<B>) -> Option<B> {
    it.find_map(f)
}

fn main() {
    println!("{}", digits_back(vec![1, 2, 3].into_iter()));
    println!("{:?} {:?}", last_even(vec![2, 4, 5].into_iter()), last_even(1..4));
    println!("{:?} {:?}", second_last(vec![7, 8, 9].into_iter()), second_last(0..1));
    println!("{:?}", second_last(vec![Some(1), None, Some(3)].into_iter()));
    let v = vec![3u32, 6, 9];
    println!("{} {:?} {:?}", v.iter().rfold(0, |a, b| a + b), v.iter().rfind(|n| **n < 7), v.iter().nth_back(2));
    let mut it = v.iter();
    println!("{:?} {:?}", it.nth_back(0), it.next_back());
    println!("{:?} {:?}", last_some(vec![Some(5), None, Some(1)].into_iter()), last_some(vec![Some(1)].into_iter()));
    println!("{} {:?}", tagged(vec![1, 2].into_iter()), last_big(vec![1, 2, 3].into_iter()));
    println!("{:?}", [5u32, 6, 7].iter().map(|n| n * 10).rfind(|n| *n < 70));
    let found: Option<Option<u32>> = first_some(vec![1, 2, 3].into_iter(), |n| (n > 1).then_some((n != 2).then_some(n)));
    println!("{found:?}");
}
