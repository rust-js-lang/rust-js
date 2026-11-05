// An `enumerate()` of std's struct, a `peekable()` or a `fuse()`, of a
// source exact-sized and double-ended, a `Vec`'s or the crate's: its
// `count()` is its source's, std's `Peekable::count`, which drops what it
// peeked at after the rest; and one stepped from its back, by a `rev()`,
// `next_back()` or `rfold`, is made knowing its back and its length where
// it's made, each index where it is (#712).
struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

/// The crate's iterator of `D`s past its first field up to its second,
/// exact-sized and double-ended.
struct Both(u32, u32);

impl Iterator for Both {
    type Item = D;
    fn next(&mut self) -> Option<D> {
        if self.0 == self.1 {
            return None;
        }
        self.0 += 1;
        Some(D(self.0))
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        let n = (self.1 - self.0) as usize;
        (n, Some(n))
    }
}

impl DoubleEndedIterator for Both {
    fn next_back(&mut self) -> Option<D> {
        if self.0 == self.1 {
            return None;
        }
        self.1 -= 1;
        Some(D(self.1 + 1))
    }
}

impl ExactSizeIterator for Both {}

fn main() {
    // Counted: what it peeked at is dropped after its source's rest.
    let mut it = vec![D(7), D(8), D(9)].into_iter().peekable();
    println!("peeked {:?}", it.peek().map(|d| d.0));
    println!("count {}", it.enumerate().count());
    let mut it = Both(0, 4).peekable();
    println!("peeked {:?}", it.peek().map(|d| d.0));
    println!("count {}", it.enumerate().count());
    println!("count {}", (1..5u32).fuse().enumerate().count());

    // Stepped from its back.
    let mut it = vec![D(1), D(2), D(3)].into_iter().peekable();
    println!("peeked {:?}", it.peek().map(|d| d.0));
    let back: Vec<(usize, u32)> = it.enumerate().rev().map(|(i, d)| (i, d.0)).collect();
    println!("{back:?}");
    let mut it = (1..5u32).peekable();
    it.peek();
    let back: Vec<(usize, u32)> = it.enumerate().rev().collect();
    println!("{back:?}");
    println!("{:?}", (1..5u32).fuse().enumerate().next_back());
    let sum = (1..5u32).fuse().enumerate().rfold(0, |sum, (i, n)| sum * 10 + i as u32 * n);
    println!("{sum}");
}
