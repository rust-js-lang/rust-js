// A `step_by()` of the crate's own fused iterator whose `next` and `nth`
// never unwind steps it by its own `nth` after its first item, as std's
// `StepBy` does, never by `next` alone: its `nth` here gives what `next`
// wouldn't, so the two differ. A chain of two such iterators the same,
// by the chain's `nth`, which asks each side's (#799). Its arithmetic is
// plain, as the corpus's native build has no overflow checks, and calls
// nothing of std's, so neither may unwind (ADR 0902).
use std::iter::FusedIterator;

struct Counter {
    at: u32,
    end: u32,
}

impl Iterator for Counter {
    type Item = u32;

    fn next(&mut self) -> Option<u32> {
        if self.at >= self.end {
            return None;
        }
        self.at += 1;
        Some(self.at)
    }

    // Its own `nth`, which marks what it gives, so a step by it shows.
    fn nth(&mut self, n: usize) -> Option<u32> {
        let left = (self.end - self.at) as usize;
        if n >= left {
            self.at = self.end;
            return None;
        }
        self.at += n as u32 + 1;
        Some(self.at + 100)
    }
}

impl FusedIterator for Counter {}

fn counter(end: u32) -> Counter {
    Counter { at: 0, end }
}

fn main() {
    let stepped: Vec<u32> = counter(10).step_by(3).collect();
    println!("{stepped:?}");
    let chained: Vec<u32> = counter(5).chain(counter(6)).step_by(4).collect();
    println!("{chained:?}");
}
