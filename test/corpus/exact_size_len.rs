// `len()` of a generic `ExactSizeIterator`: a JS iterator that knows its
// length, its `len()`, as one that knows its back is given its back (ADRs
// 0429, 0456): an array's `$iter`, a range's `$iterator` given `$rangeLen`,
// a map of one keeping it. And an iterator a variable holds, stepped or
// not, given on as it is.
fn remaining<I: ExactSizeIterator>(it: &I) -> usize {
    it.len()
}

fn both<I: DoubleEndedIterator<Item = u32> + ExactSizeIterator>(mut it: I) -> (usize, Option<u32>, usize) {
    let n = it.len();
    let last = it.next_back();
    (n, last, it.len())
}

fn mapped<I: ExactSizeIterator<Item = u32>>(it: I) -> usize {
    it.map(|n| n * 2).len()
}

fn last<I: DoubleEndedIterator<Item = u32>>(mut it: I) -> Option<u32> {
    it.next_back()
}

fn main() {
    let mut it = vec![1, 2, 3].into_iter();
    it.next();
    println!("{} {} {}", remaining(&it), remaining(&(0..5)), remaining(&(5..2)));
    println!("{:?} {:?}", both(vec![4, 5, 6].into_iter()), both(2..9));
    println!("{}", mapped([1u32, 2].into_iter()));
    println!("{:?}", last(it));
    let fresh = vec![7u32, 8].into_iter();
    println!("{:?}", last(fresh));
}
