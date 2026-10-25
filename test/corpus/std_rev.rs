// A `Rev` held is std's (ADR 0871): of the crate's iterator, stepped from
// either end, its `next_back` its source's `next`; counted, cloned and
// shown; its `nth_back`, `rfold` and `rfind`; given to generic code and
// given back from it; a reversed range held, stepped by its bounds; and one
// dropped, what it holds dropped with it. One stepped where it's made reads
// as the recognizers make it.
use std::iter::Rev;

#[derive(Clone, Debug)]
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

impl DoubleEndedIterator for Countdown {
    fn next_back(&mut self) -> Option<u32> {
        self.next().map(|n| n + 100)
    }
}

struct Loud(u8);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn reversed<I: DoubleEndedIterator>(it: I) -> Rev<I> {
    it.rev()
}

fn last_of<I: DoubleEndedIterator<Item = u32>>(mut r: Rev<I>) -> Option<u32> {
    r.next_back()
}

fn main() {
    let mut r = Countdown(4).rev();
    println!("{:?} {:?} {:?}", r.next(), r.next_back(), r.next());
    let c = r.clone();
    println!("{c:?}");
    println!("{:?}", c.collect::<Vec<_>>());

    let v = vec![1, 2, 3, 4, 5, 6];
    let mut back = v.iter().rev();
    println!("{:?} {} {:?}", back.next_back(), back.len(), back.nth_back(1));
    println!("{:?}", back.clone().rfold(0, |a, &x| a * 10 + x));
    println!("{:?} {:?}", back.rfind(|&&x| x > 3), back);

    println!("{:?}", last_of(reversed(Countdown(3))));
    println!("{:?}", last_of(reversed(v.iter().copied())));

    let mut range = (0..u32::MAX).rev();
    println!("{:?} {:?}", range.nth(4_000_000_000), range.next_back());

    let louds = vec![Loud(1), Loud(2), Loud(3)];
    let mut held = louds.into_iter().rev();
    println!("{:?}", held.next().map(|l| l.0));
    drop(held);

    let in_place: Vec<u32> = Countdown(3).rev().map(|n| n * 2).collect();
    println!("{in_place:?} {:?}", v.iter().rev().take(2).collect::<Vec<_>>());
    // A map held before it's reversed where it's stepped, of items with a
    // destructor: made knowing its back where it's made.
    let mapped = vec![Loud(4), Loud(5)].into_iter().map(|l| l);
    println!("{:?}", mapped.rev().map(|l| l.0).collect::<Vec<_>>());
    // Its own impl's, of what it's made into where it's made.
    println!("{:?} {}", v.iter().rev().next_back(), Countdown(5).rev().size_hint().0);
}
