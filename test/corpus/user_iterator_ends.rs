// The crate's double-ended, exact-sized iterator, made knowing its back and
// its length where they're asked: `enumerate().rev()`, `rposition`, and its
// adapters from either end, as std's step them (ADRs 0456, 0528).
struct Countdown {
    lo: u32,
    hi: u32,
}
impl Iterator for Countdown {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
        if self.lo < self.hi {
            self.lo += 1;
            Some(self.lo)
        } else {
            None
        }
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        let n = (self.hi - self.lo) as usize;
        (n, Some(n))
    }
}
impl DoubleEndedIterator for Countdown {
    fn next_back(&mut self) -> Option<u32> {
        if self.lo < self.hi {
            self.hi -= 1;
            Some(self.hi + 1)
        } else {
            None
        }
    }
}
impl ExactSizeIterator for Countdown {}
fn cd(n: u32) -> Countdown {
    Countdown { lo: 0, hi: n }
}
fn main() {
    println!("{:?}", cd(5).rev().collect::<Vec<_>>());
    println!("{:?}", cd(5).map(|x| x * 2).rev().collect::<Vec<_>>());
    println!("{:?}", cd(6).filter(|x| x % 2 == 0).rev().collect::<Vec<_>>());
    println!("{:?}", cd(4).zip(cd(4).rev()).collect::<Vec<_>>());
    println!("{:?}", cd(5).enumerate().rev().collect::<Vec<_>>());
    println!("{:?}", cd(7).skip(2).step_by(2).collect::<Vec<_>>());
    let mut it = cd(6);
    println!("{:?} {:?} {:?}", it.next(), it.next_back(), it.nth(1));
    println!("{:?}", it.collect::<Vec<_>>());
    println!("{:?}", cd(3).chain(cd(2)).rev().collect::<Vec<_>>());
    println!("{} {:?}", cd(4).sum::<u32>(), cd(4).max_by_key(|x| x % 3));
    println!("{:?}", cd(5).rposition(|x| x < 3));
}
