// A `Fuse` a struct holds of an iterator with a destructor, closed to drop
// it, made where its source is only an `Iterator`, stepped from its back
// where it's double-ended and exact.

struct Pad<I> {
    iter: std::iter::Fuse<I>,
    min: usize,
}

fn pad<I: Iterator>(iter: I, min: usize) -> Pad<I> {
    Pad { iter: iter.fuse(), min }
}

impl<I: Iterator> Iterator for Pad<I> {
    type Item = I::Item;

    fn next(&mut self) -> Option<I::Item> {
        self.iter.next()
    }
}

impl<I: DoubleEndedIterator + ExactSizeIterator> DoubleEndedIterator for Pad<I> {
    fn next_back(&mut self) -> Option<I::Item> {
        if self.iter.len() >= self.min {
            self.min = self.min.saturating_sub(1);
            self.iter.next_back()
        } else {
            None
        }
    }
}

struct Countdown(u32, u32);

impl Iterator for Countdown {
    type Item = u32;

    fn next(&mut self) -> Option<u32> {
        (self.0 < self.1).then(|| {
            self.0 += 1;
            self.0 - 1
        })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let n = (self.1 - self.0) as usize;
        (n, Some(n))
    }
}

impl DoubleEndedIterator for Countdown {
    fn next_back(&mut self) -> Option<u32> {
        (self.0 < self.1).then(|| {
            self.1 -= 1;
            self.1
        })
    }
}

impl ExactSizeIterator for Countdown {}

impl Drop for Countdown {
    fn drop(&mut self) {
        println!("dropped at {}..{}", self.0, self.1);
    }
}

fn main() {
    let back: Vec<i32> = pad(vec![1, 2, 3].into_iter(), 2).rev().collect();
    println!("{back:?}");
    let mut it = pad(Countdown(0, 5), 1);
    println!("{:?} {:?} {:?}", it.next(), it.next_back(), it.next_back());
    drop(it);
    let ranged: Vec<u8> = pad(0..4u8, 0).rev().collect();
    println!("{ranged:?}");
}
