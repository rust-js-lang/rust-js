// A `fuse` knows its back and its length where its source does, and an
// `enumerate` knows its length, and its back where its source knows both,
// its index from the back where the item is: of generic code's iterator
// (ADR 0528).
fn back_pairs<I: DoubleEndedIterator<Item = char> + ExactSizeIterator>(it: I) -> Vec<(usize, char)> {
    it.enumerate().rev().collect()
}

fn ends<I: DoubleEndedIterator<Item = i32> + ExactSizeIterator>(it: I) -> (Option<(usize, i32)>, Option<(usize, i32)>, usize) {
    let mut e = it.enumerate();
    let first = e.next();
    let last = e.next_back();
    (first, last, e.len())
}

fn fused_len<I: ExactSizeIterator<Item = i32>>(it: I) -> (usize, usize) {
    let mut f = it.fuse();
    f.next();
    let left = f.len();
    (left, f.count())
}

fn fused_back<I: DoubleEndedIterator<Item = i32>>(it: I) -> Vec<i32> {
    let mut f = it.fuse();
    let last = f.next_back();
    let mut rest: Vec<i32> = f.rev().collect();
    rest.extend(last);
    rest
}

fn main() {
    println!("{:?}", back_pairs(vec!['a', 'b', 'c'].into_iter()));
    println!("{:?}", ends([1, 2, 3, 4, 5].into_iter()));
    println!("{:?}", fused_len(vec![1, 2, 3].into_iter()));
    println!("{:?}", fused_back(1..5));
}
