// A type parameter bound by an iterator's extension alone, `I:
// DoubleEndedIterator` or `I: ExactSizeIterator`, is bound by `Iterator`,
// its supertrait: given as it is where a type holds it or another bound
// reads it, it's given its `Iterator` dictionary too, where it's stepped by
// it, and by it too where another of an extension's methods, `nth_back`,
// runs on a JS iterator made of it (ADR 0488).
#[derive(Clone, Debug)]
struct Countdown(u32);

impl Iterator for Countdown {
    type Item = u32;

    fn next(&mut self) -> Option<u32> {
        if self.0 == 0 {
            return None;
        }
        self.0 -= 1;
        Some(self.0)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.0 as usize, Some(self.0 as usize))
    }
}

impl ExactSizeIterator for Countdown {}

impl DoubleEndedIterator for Countdown {
    fn next_back(&mut self) -> Option<u32> {
        self.next()
    }
}

struct Ends<I> {
    inner: I,
}

impl<I: DoubleEndedIterator + ExactSizeIterator> Ends<I> {
    fn pop(&mut self) -> Option<(I::Item, usize)> {
        let item = self.inner.next_back()?;
        Some((item, self.inner.len()))
    }

    fn shift(&mut self) -> Option<I::Item> {
        self.inner.next()
    }
}

fn outer<I: DoubleEndedIterator + Clone>(mut iter: I) -> (Option<I::Item>, Option<I::Item>, usize) {
    let count = iter.clone().count();
    (iter.next(), iter.next_back(), count)
}

fn second_last<I: DoubleEndedIterator + Clone>(iter: I) -> Option<I::Item> {
    iter.clone().nth_back(1)
}

fn main() {
    let mut range = Ends { inner: 0..4 };
    println!("{:?} {:?} {:?}", range.pop(), range.shift(), range.pop());
    let mut words = Ends { inner: vec!["x", "y", "z"].into_iter() };
    println!("{:?} {:?} {:?} {:?}", words.shift(), words.pop(), words.pop(), words.shift());
    let mut own = Ends { inner: Countdown(3) };
    println!("{:?} {:?}", own.shift(), own.pop());

    println!("{:?}", outer(vec![1, 2, 3].into_iter()));
    println!("{:?}", outer(5..8));
    println!("{:?}", outer(Countdown(2)));

    println!("{:?} {:?}", second_last(5..9), second_last(vec!['p', 'q'].into_iter()));
    println!("{:?}", second_last(Countdown(1)));
}
