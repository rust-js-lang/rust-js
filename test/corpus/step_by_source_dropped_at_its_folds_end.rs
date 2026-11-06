// A `step_by()` of the crate's iterator folded by value drops its source
// as the fold ends, not as it's stepped to its end: where that drop
// panics, the accumulator, or `last`'s item, is the fold's result, which
// rustc leaks as a later drop unwinds (rust-lang/rust#47949, #845).
use std::panic::catch_unwind;

struct Acc(u32);

impl Drop for Acc {
    fn drop(&mut self) {
        println!("drop acc {}", self.0);
    }
}

struct It {
    at: u32,
    end: u32,
}

impl Iterator for It {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
        if self.at == self.end {
            return None;
        }
        self.at += 1;
        Some(self.at)
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        let n = (self.end - self.at) as usize;
        (n, Some(n))
    }
}

impl DoubleEndedIterator for It {
    fn next_back(&mut self) -> Option<u32> {
        if self.at == self.end {
            return None;
        }
        self.end -= 1;
        Some(self.end + 1)
    }
    // Its own, which a `step_by()`'s `rfold` asks of it.
    fn nth_back(&mut self, n: usize) -> Option<u32> {
        for _ in 0..n {
            self.next_back()?;
        }
        self.next_back()
    }
}

impl ExactSizeIterator for It {}

impl Drop for It {
    fn drop(&mut self) {
        println!("drop it");
        panic!("drop");
    }
}

fn it() -> It {
    It { at: 0, end: 5 }
}

fn main() {
    let r = catch_unwind(|| {
        let acc = it().step_by(2).fold(Acc(0), |mut a, x| {
            a.0 += x;
            a
        });
        println!("fold {}", acc.0);
    });
    println!("{}", r.is_err());
    let r = catch_unwind(|| {
        let acc = it().step_by(2).rfold(Acc(0), |mut a, x| {
            a.0 += x;
            a
        });
        println!("rfold {}", acc.0);
    });
    println!("{}", r.is_err());
    let r = catch_unwind(|| {
        let l = it().step_by(2).map(Acc).last();
        println!("last {}", l.map_or(0, |a| a.0));
    });
    println!("{}", r.is_err());
}
