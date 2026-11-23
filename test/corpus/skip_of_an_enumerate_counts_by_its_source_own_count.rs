// A `skip()` of an `enumerate()` counts as std's `Skip::count` does: it
// steps past what it skips, then asks the rest's `count`, `Enumerate`'s,
// which asks its source's, the crate's own, which prints. Of a source the
// enumeration is made in place of, knowing its back where its own
// `next_back` is called, the runtime's `enumerate` says its count is its
// source's (#671, #1979).
struct Tally {
    lo: u32,
    hi: u32,
}

impl Iterator for Tally {
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
    fn count(self) -> usize {
        println!("own count of {}", self.hi - self.lo);
        (self.hi - self.lo) as usize
    }
}

impl DoubleEndedIterator for Tally {
    fn next_back(&mut self) -> Option<u32> {
        if self.lo < self.hi {
            self.hi -= 1;
            Some(self.hi + 1)
        } else {
            None
        }
    }
}

impl ExactSizeIterator for Tally {}

fn tally(hi: u32) -> Tally {
    Tally { lo: 0, hi }
}

fn main() {
    let mut held = tally(4);
    println!("{:?} {:?}", held.next_back(), held.next());
    println!("{}", tally(5).enumerate().skip(2).count());
    println!("{}", tally(5).enumerate().count());
    println!("{:?}", tally(3).enumerate().rev().collect::<Vec<_>>());
}
