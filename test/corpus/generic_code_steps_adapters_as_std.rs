// What generic code does with an iterator it's given is what std does: a
// collect into an `Option` steps a `skip(1)` by its `try_fold`, which asks
// its source's `nth(0)`, not its `next`'s `nth(1)` (#1004); and `last()` of
// a `flat_map` is std's `FlattenCompat::last`, each inner iterator's own
// `last`, the inner stepped from the back taken last, dropping what each
// passes as std's does (#645). The iterator reaches the generic function
// as generic code's caller makes it, its steps passed on (#2051).
struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

struct Counting {
    at: u32,
    end: u32,
}

impl Iterator for Counting {
    type Item = u32;

    fn next(&mut self) -> Option<u32> {
        println!("next");
        if self.at == self.end {
            return None;
        }
        self.at += 1;
        Some(self.at)
    }

    fn nth(&mut self, n: usize) -> Option<u32> {
        println!("nth {n}");
        for _ in 0..n {
            self.next()?;
        }
        self.next()
    }
}

/// `Counting`, stepped from either end, its length known, so std's `Skip`
/// of it is double-ended, and a `flat_map` of those too.
struct Both {
    at: u32,
    end: u32,
}

impl Iterator for Both {
    type Item = u32;

    fn next(&mut self) -> Option<u32> {
        println!("next");
        if self.at == self.end {
            return None;
        }
        self.at += 1;
        Some(self.at)
    }

    fn nth(&mut self, n: usize) -> Option<u32> {
        println!("nth {n}");
        for _ in 0..n {
            self.next()?;
        }
        self.next()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let left = (self.end - self.at) as usize;
        (left, Some(left))
    }
}

impl DoubleEndedIterator for Both {
    fn next_back(&mut self) -> Option<u32> {
        println!("next_back");
        if self.at == self.end {
            return None;
        }
        self.end -= 1;
        Some(self.end + 1)
    }
}

impl ExactSizeIterator for Both {}

/// A collect of what may be stepped from either end, as generic code that
/// asks it for a `DoubleEndedIterator` is given it.
fn collected_of_both<I: DoubleEndedIterator, B: FromIterator<I::Item>>(it: I) -> B {
    it.collect()
}

fn back_then_last<I: DoubleEndedIterator>(mut it: I) -> Option<I::Item> {
    drop(it.next_back());
    it.last()
}

fn collected<I: Iterator, B: FromIterator<I::Item>>(it: I) -> B {
    it.collect()
}

fn last<I: Iterator>(it: I) -> Option<I::Item> {
    it.last()
}

fn main() {
    let skipped: Option<Vec<u32>> = collected(Counting { at: 0, end: 4 }.skip(1).map(Some));
    println!("{skipped:?}");
    let flattened: Option<Vec<u32>> =
        collected_of_both(Both { at: 0, end: 3 }.skip(1).flat_map(|n| vec![Some(n), Some(n * 10)]));
    println!("{flattened:?}");
    let found = last((0..3).flat_map(|i| vec![D(i + 20), D(i + 30)]));
    println!("last {}", found.as_ref().map_or(0, |d| d.0));
    drop(found);
    let mut both = (0..3).flat_map(|i| vec![D(i + 40), D(i + 50)]);
    let back = both.next_back();
    println!("back {}", back.as_ref().map_or(0, |d| d.0));
    drop(back);
    let found = last(both);
    println!("last {}", found.as_ref().map_or(0, |d| d.0));
    drop(found);
    let found = last(
        (0..3)
            .flat_map(|i| vec![D(i + 60), D(i + 70)])
            .chain(std::iter::empty()),
    );
    println!("last {}", found.as_ref().map_or(0, |d| d.0));
    drop(found);
    let found = back_then_last((0..3).flat_map(|i| vec![D(i + 80), D(i + 90)]));
    println!("last {}", found.as_ref().map_or(0, |d| d.0));
}
