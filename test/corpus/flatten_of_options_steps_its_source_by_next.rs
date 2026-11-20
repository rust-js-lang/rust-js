// A `flatten()` of `Option`s, or a `flat_map` to them, is std's
// `FlattenCompat` of one-shot inner iterators, which steps its source by
// `next` and `next_back` (#2205). A `skip(n)` between asks the crate's own
// `nth(n)` as its `next`, where its `try_fold`, which a filter of the
// `Some`s steps it by, asks `nth(n - 1)` and then `next`; a `step_by`'s
// first `next` asks `nth(0)`. Where the crate asks the hint of its
// iterator's type elsewhere, a `zip`'s `len()` here, each one is made
// knowing its hint, so the flatten is made in place. Its hint is std's
// one-shot one, none to its source's upper bound, which generic code given
// one asks, as it asks its `next`. A boxed iterator of the crate's with its
// own `nth` and no `size_hint` is made in place with no hint fact: its own
// `nth` alone chooses std's `Flatten` there.

/// Counts up to `.1` from `.0`, from either end, with its own `nth`,
/// `nth_back` and `size_hint`.
struct Own(u32, u32);

impl Iterator for Own {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
        println!("own next");
        if self.0 == self.1 {
            return None;
        }
        self.0 += 1;
        Some(self.0)
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        let n = (self.1 - self.0) as usize;
        println!("own size_hint {n}");
        (n, Some(n))
    }
    fn nth(&mut self, n: usize) -> Option<u32> {
        println!("own nth({n})");
        self.0 = (self.0 + n as u32).min(self.1);
        self.next()
    }
}

impl DoubleEndedIterator for Own {
    fn next_back(&mut self) -> Option<u32> {
        println!("own next_back");
        if self.0 == self.1 {
            return None;
        }
        self.1 -= 1;
        Some(self.1 + 1)
    }
    fn nth_back(&mut self, n: usize) -> Option<u32> {
        println!("own nth_back({n})");
        self.1 = self.1.saturating_sub(n as u32).max(self.0);
        self.next_back()
    }
}

impl ExactSizeIterator for Own {}

/// Counts up to `.1` from `.0`, with its own `nth` and no `size_hint`.
struct Counter(u32, u32);

impl Iterator for Counter {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
        println!("counter next");
        if self.0 == self.1 {
            return None;
        }
        self.0 += 1;
        Some(self.0)
    }
    fn nth(&mut self, n: usize) -> Option<u32> {
        println!("counter nth({n})");
        self.0 = (self.0 + n as u32).min(self.1);
        self.next()
    }
}

fn first<I: Iterator<Item = u32>>(mut items: I) -> Option<u32> {
    items.next()
}

fn hint<I: Iterator<Item = u32>>(items: I) -> (usize, Option<usize>) {
    items.size_hint()
}

fn main() {
    let zipped = Own(0, 3).zip(10..20u32);
    println!("zip len {}", zipped.len());

    println!("{:?}", Own(0, 5).skip(1).map(Some).flatten().next());
    println!("{:?}", Own(0, 5).skip(1).flat_map(Some).next());
    println!("{:?}", Own(0, 5).step_by(2).map(Some).flatten().next());
    println!("{:?}", Own(0, 5).skip(1).map(Some).flatten().max_by(|a, b| a.cmp(b)));
    println!("{:?}", Own(0, 5).skip(1).map(Some).flatten().nth(1));
    println!("{:?}", Own(0, 5).skip(1).map(Some).flatten().size_hint());
    println!("{:?}", Own(0, 5).skip(1).map(Some).flatten().next_back());
    println!("{:?}", Own(0, 5).rev().skip(1).map(Some).flatten().next());
    println!("{:?}", first(Own(0, 5).skip(1).map(Some).flatten()));
    println!("{:?}", Box::new(Counter(0, 5)).skip(1).map(Some).flatten().next());
    println!("{:?}", Box::new(Counter(0, 5)).step_by(2).flat_map(Some).next());
    println!("{:?}", hint(Own(0, 5).skip(1).map(Some).flatten()));
    println!("{:?}", hint(Own(0, 5).step_by(2).flat_map(Some)));
}
