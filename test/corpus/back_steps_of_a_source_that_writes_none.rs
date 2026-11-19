// A `take`'s, an `enumerate`'s and a `skip`'s back steps are std's whatever
// their source writes: std's `Take::rfold` asks its source's `nth_back`
// past what it won't give, then its source's `rfold`, each the trait's
// default where the source writes only `next`, `next_back` and
// `size_hint`, so its length is asked once, not at each step (#2136);
// `Enumerate::rfold`, `try_rfold` and `nth_back` ask it once too, and
// `nth_back` none where it finds none (#2140, #738, #834), and
// `Skip::nth_back` (#2141). Generic code over the crate's iterator.

struct Bare(Vec<u32>);

impl Iterator for Bare {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
        println!("next");
        if self.0.is_empty() { None } else { Some(self.0.remove(0)) }
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        println!("hint");
        (self.0.len(), Some(self.0.len()))
    }
}

impl DoubleEndedIterator for Bare {
    fn next_back(&mut self) -> Option<u32> {
        println!("back");
        self.0.pop()
    }
}

impl ExactSizeIterator for Bare {}

/// One that writes its own `nth_back` and `len`, but none of the folds.
struct NoFold(Vec<u32>);

impl Iterator for NoFold {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
        println!("next");
        if self.0.is_empty() { None } else { Some(self.0.remove(0)) }
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.0.len(), Some(self.0.len()))
    }
}

impl DoubleEndedIterator for NoFold {
    fn next_back(&mut self) -> Option<u32> {
        println!("back");
        self.0.pop()
    }
    fn nth_back(&mut self, n: usize) -> Option<u32> {
        println!("own nth_back {n}");
        let keep = self.0.len().saturating_sub(n);
        self.0.truncate(keep);
        self.0.pop()
    }
}

impl ExactSizeIterator for NoFold {
    fn len(&self) -> usize {
        println!("len");
        self.0.len()
    }
}

fn taken<I: DoubleEndedIterator<Item = u32> + ExactSizeIterator>(src: I) -> u32 {
    src.take(3).rfold(0, |sum, x| sum * 2 + x)
}

fn taken_found<I: DoubleEndedIterator<Item = u32> + ExactSizeIterator>(src: I) -> Option<u32> {
    src.take(3).rfind(|x| x % 2 == 1)
}

fn taken_nth_back<I: DoubleEndedIterator<Item = u32> + ExactSizeIterator>(src: I) -> Option<u32> {
    src.take(3).nth_back(1)
}

fn enumerated<I: DoubleEndedIterator<Item = u32> + ExactSizeIterator>(src: I) -> usize {
    src.enumerate().rfold(0, |sum, (i, x)| sum * 2 + i + x as usize)
}

fn enumerated_nth_back<I: DoubleEndedIterator<Item = u32> + ExactSizeIterator>(src: I) -> Option<(usize, u32)> {
    src.enumerate().nth_back(1)
}

fn enumerated_found<I: DoubleEndedIterator<Item = u32> + ExactSizeIterator>(src: I) -> Option<(usize, u32)> {
    src.enumerate().rev().find(|(i, _)| i % 2 == 0)
}

fn enumerated_past_its_end<I: DoubleEndedIterator<Item = u32> + ExactSizeIterator>(src: I) -> Option<(usize, u32)> {
    src.enumerate().rev().nth(5)
}

fn skipped_nth_back<I: DoubleEndedIterator<Item = u32> + ExactSizeIterator>(src: I) -> Option<u32> {
    src.skip(1).nth_back(1)
}

fn skipped<I: DoubleEndedIterator<Item = u32> + ExactSizeIterator>(src: I) -> u32 {
    src.skip(1).rfold(0, |sum, x| sum * 2 + x)
}

fn main() {
    println!("{}", taken(Bare(vec![1, 2, 3, 4, 5])));
    println!("{}", taken(NoFold(vec![1, 2, 3, 4, 5])));
    println!("{:?}", taken_found(Bare(vec![1, 2, 3, 4, 5])));
    println!("{:?}", taken_nth_back(Bare(vec![1, 2, 3, 4, 5])));
    println!("{}", enumerated(Bare(vec![1, 2, 3])));
    println!("{}", enumerated(NoFold(vec![1, 2, 3])));
    println!("{:?}", enumerated_nth_back(Bare(vec![1, 2, 3])));
    println!("{:?}", enumerated_past_its_end(Bare(vec![1, 2, 3])));
    println!("{:?}", enumerated_found(Bare(vec![1, 2, 3, 4])));
    println!("{:?}", skipped_nth_back(Bare(vec![1, 2, 3, 4])));
    println!("{:?}", skipped_nth_back(NoFold(vec![1, 2, 3, 4])));
    println!("{}", skipped(Bare(vec![1, 2, 3, 4])));
}
