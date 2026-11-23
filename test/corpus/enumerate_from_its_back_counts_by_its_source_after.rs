// An `enumerate()` stepped from its back numbers each item its count plus
// the length its source says after that step, as std's `Enumerate::next_back`
// asks it: of a crate's own iterator whose `size_hint` doesn't shrink, which
// `ExactSizeIterator` asks of it but nothing checks, every item is 10, where
// asking before the step gave 9. Its own `next_back`, called where it's held,
// has the enumeration made in place, knowing its back (ADR 0456, #1979).
struct Stuck {
    lo: u32,
    hi: u32,
}

impl Iterator for Stuck {
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
        (10, Some(10))
    }
}

impl DoubleEndedIterator for Stuck {
    fn next_back(&mut self) -> Option<u32> {
        if self.lo < self.hi {
            self.hi -= 1;
            Some(self.hi + 1)
        } else {
            None
        }
    }
}

impl ExactSizeIterator for Stuck {}

fn stuck(hi: u32) -> Stuck {
    Stuck { lo: 0, hi }
}

fn main() {
    let mut held = stuck(4);
    println!("{:?} {:?}", held.next_back(), held.next());
    println!("{:?}", stuck(3).enumerate().rev().collect::<Vec<_>>());
    let mut steps = stuck(2).enumerate();
    println!("{:?} {:?} {:?}", steps.next_back(), steps.next(), steps.next_back());
}
