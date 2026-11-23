// A `Box` of an adapter of the crate's iterator given to generic code that
// asks its length and steps it from its back is made knowing them, as one
// given unboxed is, so what std's `Box<I>` passes on reaches it: `len()`,
// `size_hint()`, `next_back()`, `nth_back()` and `rfold()`, the crate's own
// `size_hint` and `next_back` run (#2419).
struct Every(u32, u32);

impl Iterator for Every {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
        (self.0 < self.1).then(|| {
            self.0 += 1;
            self.0
        })
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        let left = (self.1 - self.0) as usize;
        println!("own size_hint {left}");
        (left, Some(left))
    }
}

impl ExactSizeIterator for Every {}

impl DoubleEndedIterator for Every {
    fn next_back(&mut self) -> Option<u32> {
        (self.0 < self.1).then(|| {
            self.1 -= 1;
            println!("own next_back {}", self.1 + 1);
            self.1 + 1
        })
    }
}

fn hinted<I: DoubleEndedIterator<Item = u32> + ExactSizeIterator>(it: I) {
    println!("hint {:?} len {}", it.size_hint(), it.len());
}

fn from_the_back<I: DoubleEndedIterator<Item = u32> + ExactSizeIterator>(mut it: I) {
    println!("back {:?} then {:?}", it.next_back(), it.nth_back(1));
    println!("rest {}", it.rfold(String::new(), |s, n| format!("{s}{n},")));
}

fn main() {
    hinted(Box::new(Every(0, 5).map(|x| x + 100)));
    from_the_back(Box::new(Every(0, 6).map(|x| x * 2)));
    from_the_back(Box::new(Box::new(Every(0, 4).rev())));
}
