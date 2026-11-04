// Generic code's `rev()` of what it's given: its `nth` is its source's
// `nth_back`, the crate's own where it writes one, as std's `Rev::nth`
// is, so a `skip` or a `step_by` over it, which step by `nth`, ask it, and
// it drops what it passes itself (#649).
struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

/// The crate's iterator of `D(a + 1)` to `D(b)`, which writes its own
/// `nth_back`.
struct Backs(u32, u32);

impl Drop for Backs {
    fn drop(&mut self) {
        println!("drop backs");
    }
}

impl Iterator for Backs {
    type Item = D;
    fn next(&mut self) -> Option<D> {
        (self.0 < self.1).then(|| {
            self.0 += 1;
            D(self.0)
        })
    }
}

impl DoubleEndedIterator for Backs {
    fn next_back(&mut self) -> Option<D> {
        (self.0 < self.1).then(|| {
            self.1 -= 1;
            D(self.1 + 1)
        })
    }
    fn nth_back(&mut self, n: usize) -> Option<D> {
        println!("own nth_back {n}");
        for _ in 0..n {
            self.next_back()?;
        }
        self.next_back()
    }
}

fn skipped<I: DoubleEndedIterator<Item = D>>(it: I) -> Vec<u32> {
    it.rev().skip(2).map(|d| d.0).collect()
}

fn skipped_last<I: DoubleEndedIterator<Item = D>>(it: I) -> Option<u32> {
    it.rev().skip(1).last().map(|d| d.0)
}

fn stepped<I: DoubleEndedIterator<Item = D>>(it: I) -> Vec<u32> {
    it.rev().step_by(2).map(|d| d.0).collect()
}

fn stepped_count<I: DoubleEndedIterator<Item = D>>(it: I) -> usize {
    it.rev().step_by(2).count()
}

fn main() {
    println!("skipped {:?}", skipped(Backs(0, 5)));
    println!("skipped last {:?}", skipped_last(Backs(0, 5)));
    println!("stepped {:?}", stepped(Backs(0, 5)));
    println!("stepped count {}", stepped_count(Backs(0, 5)));

    // A `Vec`'s, which writes none of its own.
    let ds = || vec![D(1), D(2), D(3), D(4), D(5)];
    println!("skipped {:?}", skipped(ds().into_iter()));
    println!("stepped {:?}", stepped(ds().into_iter()));

    // Not generic: the crate's own, as before.
    let direct: Vec<u32> = Backs(0, 5).rev().skip(2).map(|d| d.0).collect();
    println!("direct {direct:?}");
}
