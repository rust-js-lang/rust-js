// `size_hint()`: an iterator of the crate's own, its own or std's `(0, None)`,
// one passing on another's, as utf8_iter's does, and std's of an iterator
// that knows its length, whose hint is exact.

struct Countdown(u32);

impl Iterator for Countdown {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
        if self.0 == 0 {
            None
        } else {
            self.0 -= 1;
            Some(self.0)
        }
    }
}

struct Known(u32);

impl Iterator for Known {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
        self.0 = self.0.checked_sub(1)?;
        Some(self.0)
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.0 as usize, Some(self.0 as usize))
    }
}

struct Indexed {
    inner: Countdown,
    at: usize,
}

impl Iterator for Indexed {
    type Item = (usize, u32);
    fn next(&mut self) -> Option<(usize, u32)> {
        let item = self.inner.next()?;
        self.at += 1;
        Some((self.at - 1, item))
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

fn main() {
    println!("{:?} {:?}", Countdown(3).size_hint(), Known(4).size_hint());
    let mut indexed = Indexed { inner: Countdown(2), at: 0 };
    println!("{:?} {:?}", indexed.size_hint(), indexed.next());

    let v = vec![1, 2, 3];
    let mut it = v.iter();
    it.next();
    println!("{:?}", it.size_hint());
    println!("{:?}", v.iter().map(|x| x * 2).size_hint());
    println!("{:?} {:?}", [1, 2].into_iter().size_hint(), v.clone().into_iter().size_hint());
}
