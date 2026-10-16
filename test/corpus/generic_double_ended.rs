// A generic `T: DoubleEndedIterator`, and a `dyn` of one, is a JS iterator
// that knows its back: an array's `$iter`, or `$iterator(it, next, false,
// nextBack)` of a range or of one of the crate's own, which `next_back()`
// steps from its end, as Rust's does.

struct Span {
    low: u32,
    high: u32,
}

impl Iterator for Span {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
        if self.low >= self.high {
            return None;
        }
        self.low += 1;
        Some(self.low - 1)
    }
}

impl DoubleEndedIterator for Span {
    fn next_back(&mut self) -> Option<u32> {
        if self.low >= self.high {
            return None;
        }
        self.high -= 1;
        Some(self.high)
    }
}

fn pairwise_sub<T: DoubleEndedIterator<Item = u32>>(mut t: T) -> u32 {
    let mut result = 0;
    while let (Some(front), Some(back)) = (t.next(), t.next_back()) {
        result += back - front;
    }
    result
}

fn last_then_rest<T: DoubleEndedIterator<Item = u32>>(mut t: T) -> (Option<u32>, Vec<u32>) {
    let last = t.next_back();
    (last, t.collect())
}

fn reversed<T: DoubleEndedIterator<Item = u32>>(t: T) -> Vec<u32> {
    t.rev().collect()
}

fn passed_on<T: DoubleEndedIterator<Item = u32>>(t: T) -> u32 {
    pairwise_sub(t)
}

fn ends(t: &mut dyn DoubleEndedIterator<Item = u32>) -> (Option<u32>, Option<u32>) {
    (t.next(), t.next_back())
}

fn boxed(mut t: Box<dyn DoubleEndedIterator<Item = u32>>) -> Vec<u32> {
    let mut out = Vec::new();
    while let Some(back) = t.next_back() {
        out.push(back);
        if let Some(front) = t.next() {
            out.push(front);
        }
    }
    out
}

fn maybe<T: DoubleEndedIterator<Item = Option<u32>>>(mut t: T) -> (Option<Option<u32>>, Option<Option<u32>>) {
    (t.next_back(), t.next())
}

fn main() {
    let v = vec![1, 2, 3, 4, 5, 6];
    println!("{}", pairwise_sub(v.into_iter()));
    println!("{}", pairwise_sub(0..10));
    println!("{}", pairwise_sub(Span { low: 3, high: 9 }));
    println!("{}", passed_on([5, 1, 9].into_iter()));
    println!("{:?}", last_then_rest(vec![7, 8, 9].into_iter()));
    println!("{:?}", last_then_rest(1..=4));
    println!("{:?}", reversed(Span { low: 0, high: 4 }));
    println!("{:?}", reversed(vec![1, 2, 3].into_iter()));
    let mut range = 10..14;
    println!("{:?}", ends(&mut range));
    println!("{:?}", range);
    println!("{:?}", ends(&mut Span { low: 1, high: 2 }));
    println!("{:?}", boxed(Box::new(vec![1, 2, 3, 4, 5].into_iter())));
    println!("{:?}", boxed(Box::new(0..3)));
    println!("{:?}", boxed(Box::new(Span { low: 4, high: 7 })));
    println!("{:?}", maybe(vec![None, Some(1)].into_iter()));
    println!("{:?}", maybe(vec![Some(2), None].into_iter()));
}
