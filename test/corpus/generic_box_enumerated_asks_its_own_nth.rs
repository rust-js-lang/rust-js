// Generic code's sized `Box<I>` of a double-ended, exact-sized iterator,
// lent or owned, knows its back and its length as `I` does, as std's
// `Box<I>` passes them on: its `enumerate().skip(n)` asks the crate's own
// `nth`, through the `&mut` and the box, where it skipped by `next`, and
// its `enumerate().rev()`, which steps by its length, compiles, where it
// was refused (#2227).
struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

struct Every(u32);

impl Iterator for Every {
    type Item = D;

    fn next(&mut self) -> Option<D> {
        println!("own next");
        self.0 = self.0.checked_sub(1)?;
        Some(D(self.0))
    }

    fn nth(&mut self, n: usize) -> Option<D> {
        println!("own nth {n}");
        for _ in 0..n {
            self.next()?;
        }
        self.next()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.0 as usize, Some(self.0 as usize))
    }
}

impl DoubleEndedIterator for Every {
    fn next_back(&mut self) -> Option<D> {
        println!("own next_back");
        self.next()
    }
}

impl ExactSizeIterator for Every {}

impl Drop for Every {
    fn drop(&mut self) {
        println!("drop Every {}", self.0);
    }
}

fn lent<I: DoubleEndedIterator<Item = D> + ExactSizeIterator>(src: &mut Box<I>) {
    for (i, x) in src.enumerate().skip(1) {
        println!("lent {i} {}", x.0);
    }
}

fn owned<I: DoubleEndedIterator<Item = D> + ExactSizeIterator>(src: Box<I>) {
    for (i, x) in src.enumerate().skip(2) {
        println!("owned {i} {}", x.0);
    }
}

fn reversed<I: DoubleEndedIterator<Item = D> + ExactSizeIterator>(src: Box<I>) {
    for (i, x) in src.enumerate().rev().skip(1) {
        println!("reversed {i} {}", x.0);
    }
}

fn main() {
    for n in [0, 1, 4] {
        let mut b = Box::new(Every(n));
        lent(&mut b);
        println!("left {}", b.len());
        owned(Box::new(Every(n)));
        reversed(Box::new(Every(n)));
    }
}
