//@ library-differs
// An `enumerate()` of the crate's iterator counts by that iterator's own
// `count`, as std's `Enumerate::count` asks it, wherever something of its
// type is stepped from its back: the same value before (#2153's second
// shape), another value of its type, or a generic
// `impl<I: DoubleEndedIterator> DoubleEndedIterator for Own<I>` given one
// that isn't double-ended, whose `next_back` never runs on it (#2153). And
// a value moved into `Own` whose type isn't double-ended leaves another of
// its type, a held `map`, std's compiled struct.
struct Count(u32);

impl Iterator for Count {
    type Item = u32;

    fn next(&mut self) -> Option<u32> {
        if self.0 < 5 {
            self.0 += 1;
            Some(self.0)
        } else {
            None
        }
    }

    fn count(self) -> usize {
        println!("own count from {}", self.0);
        100 + self.0 as usize
    }

    fn fold<B, F: FnMut(B, u32) -> B>(mut self, init: B, mut f: F) -> B {
        println!("own fold from {}", self.0);
        let mut acc = init;
        while let Some(x) = self.next() {
            acc = f(acc, x);
        }
        acc
    }
}

struct Ends(u32, u32);

impl Iterator for Ends {
    type Item = u32;

    fn next(&mut self) -> Option<u32> {
        if self.0 < self.1 {
            self.0 += 1;
            Some(self.0)
        } else {
            None
        }
    }

    fn count(self) -> usize {
        println!("ends' own count of {}..{}", self.0, self.1);
        (self.1 - self.0) as usize
    }
}

impl DoubleEndedIterator for Ends {
    fn next_back(&mut self) -> Option<u32> {
        if self.0 < self.1 {
            self.1 -= 1;
            Some(self.1 + 1)
        } else {
            None
        }
    }
}

pub struct Own<I>(pub I);

impl<I: Iterator> Iterator for Own<I> {
    type Item = I::Item;

    fn next(&mut self) -> Option<I::Item> {
        self.0.next()
    }
}

impl<I: DoubleEndedIterator> DoubleEndedIterator for Own<I> {
    fn next_back(&mut self) -> Option<I::Item> {
        self.0.next_back()
    }
}

fn counted<I: Iterator>(it: I) -> usize {
    it.count()
}

fn double(x: u32) -> u32 {
    x * 2
}

fn main() {
    // `Own<Count>` isn't double-ended: its `next_back` never steps `Count`.
    println!("{}", Own(Count(0)).count());
    println!("{}", Count(0).enumerate().count());
    println!("{}", counted(Count(1).enumerate()));

    // `Ends` is: stepped from its back, then counted through `enumerate()`.
    let mut ends = Ends(0, 6);
    println!("{:?}", ends.next_back());
    println!("{}", ends.enumerate().count());
    // Another `Ends`, beside one stepped from its back.
    let mut other = Ends(2, 9);
    println!("{:?}", other.next_back());
    println!("{}", Ends(1, 4).enumerate().count());

    // A held map of `Count`, moved into `Own`, and another of its type.
    let first = Count(0).map(double as fn(u32) -> u32);
    let second = Count(1).map(double as fn(u32) -> u32);
    println!("{}", Own(first).count());
    println!("{}", second.enumerate().count());
}
