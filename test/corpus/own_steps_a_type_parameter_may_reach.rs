// What a type parameter's dictionary says of its own steps, which generic
// code's adapters of it call, and whether one took it: a `fold` or an
// `rfold` its value's own may be, the crate's, called through a `map()`, a
// `fuse()` or a `peekable()` of it, and dropped once, by that step where it
// took it, else after it (#621, #623, #649). A compiled `Peekable`'s `rfold`
// is std's, which drops its source before what it peeked at, and once as a
// panic in its closure unwinds.
struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

/// The crate's iterator that writes its own `fold` and `rfold`.
struct Folds(u32, u32);

impl Drop for Folds {
    fn drop(&mut self) {
        println!("drop folds");
    }
}

impl Iterator for Folds {
    type Item = D;
    fn next(&mut self) -> Option<D> {
        (self.0 < self.1).then(|| {
            self.0 += 1;
            D(self.0)
        })
    }
    fn fold<B, F: FnMut(B, D) -> B>(mut self, init: B, mut f: F) -> B {
        println!("own fold");
        let mut acc = init;
        while let Some(x) = self.next() {
            acc = f(acc, x);
        }
        acc
    }
}

impl DoubleEndedIterator for Folds {
    fn next_back(&mut self) -> Option<D> {
        (self.0 < self.1).then(|| {
            self.1 -= 1;
            D(self.1 + 1)
        })
    }
    fn rfold<B, F: FnMut(B, D) -> B>(mut self, init: B, mut f: F) -> B {
        println!("own rfold");
        let mut acc = init;
        while let Some(x) = self.next_back() {
            acc = f(acc, x);
        }
        acc
    }
}

/// One that writes neither.
struct Steps(u32, u32);

impl Drop for Steps {
    fn drop(&mut self) {
        println!("drop steps");
    }
}

impl Iterator for Steps {
    type Item = D;
    fn next(&mut self) -> Option<D> {
        (self.0 < self.1).then(|| {
            self.0 += 1;
            D(self.0)
        })
    }
}

impl DoubleEndedIterator for Steps {
    fn next_back(&mut self) -> Option<D> {
        (self.0 < self.1).then(|| {
            self.1 -= 1;
            D(self.1 + 1)
        })
    }
}

fn mapped_fused<I: Iterator<Item = D>>(it: I) -> u32 {
    it.map(|d| D(d.0 + 10)).fuse().fold(0, |sum, d| sum + d.0)
}

fn peeked_reversed<I: DoubleEndedIterator<Item = D>>(it: I) -> u32 {
    let mut it = it.peekable();
    println!("peeked {:?}", it.peek().map(|d| d.0));
    it.rev().fold(0, |sum, d| sum + d.0)
}

fn fused_total<I: Iterator<Item = u32>>(it: I) -> u32 {
    let total = it.fuse().fold(D(0), |mut total, n| {
        total.0 += n;
        total
    });
    total.0
}

/// One whose `next_back` panics at its third item from the back.
struct Breaks(u32, u32);

impl Drop for Breaks {
    fn drop(&mut self) {
        println!("drop breaks");
    }
}

impl Iterator for Breaks {
    type Item = D;
    fn next(&mut self) -> Option<D> {
        (self.0 < self.1).then(|| {
            self.0 += 1;
            D(self.0)
        })
    }
}

impl DoubleEndedIterator for Breaks {
    fn next_back(&mut self) -> Option<D> {
        if self.1 == 2 {
            panic!("next_back");
        }
        (self.0 < self.1).then(|| {
            self.1 -= 1;
            D(self.1 + 1)
        })
    }
}

/// One that counts, with a destructor and no `fold` of its own.
struct Counter(u32);

impl Iterator for Counter {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
        (self.0 < 3).then(|| {
            self.0 += 1;
            self.0
        })
    }
}

impl Drop for Counter {
    fn drop(&mut self) {
        println!("counter dropped at {}", self.0);
    }
}

/// What holds an iterator and sums it, through its dictionary.
struct Holder<I> {
    it: I,
}

impl<I: Iterator<Item = u32>> Holder<I> {
    fn total(self) -> u32 {
        self.it.sum()
    }
}

fn held_total<I: Iterator<Item = u32>>(it: I) -> u32 {
    Holder { it }.total()
}

fn held_mapped_total<I: Iterator<Item = u32>>(it: I) -> u32 {
    Holder { it: it.map(|n| n + 1) }.total()
}

fn total_of<I: Iterator<Item = D>>(it: I) -> u32 {
    it.fold(0, |total, d| total + d.0)
}

fn main() {
    println!("{}", mapped_fused(Folds(0, 2)));
    println!("{}", mapped_fused(Steps(0, 2)));
    println!("{}", mapped_fused(vec![D(1), D(2)].into_iter()));
    println!("{}", peeked_reversed(Folds(0, 3)));
    println!("{}", peeked_reversed(Steps(0, 3)));
    println!("{}", peeked_reversed(vec![D(1), D(2), D(3)].into_iter()));
    let mut it = Steps(0, 3).peekable();
    println!("peeked {:?}", it.peek().map(|d| d.0));
    println!("{}", it.rev().fold(0, |sum, d| sum + d.0));
    // Std's `rfold`, generic code's `fold` of a `rev()`, whose source's
    // `next_back` panics drops what it held as it unwinds, once.
    let folded = std::panic::catch_unwind(|| {
        let mut it = Breaks(0, 4).peekable();
        println!("peeked {:?}", it.peek().map(|d| d.0));
        total_of(it.rev())
    });
    println!("panicked {}", folded.is_err());
    let names = ["a", "bb", "ccc"];
    println!("{}", held_total(Counter(0)));
    println!("{}", held_mapped_total(Counter(0)));
    println!("{}", fused_total(names.iter().map(|name| name.len() as u32)));
}
