// What a type parameter's dictionary says of its own steps, which generic
// code's adapters of it call, and whether one took it: a `fold` or an
// `rfold` its value's own may be, the crate's, called through a `map()`, a
// `fuse()` or a `peekable()` of it, and dropped once, by that step where it
// took it, else after it (#621, #623, #649). A compiled `Peekable`'s `rfold`
// is std's, which drops its source before what it peeked at.
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

fn fused_sum<I: Iterator<Item = u32>>(it: I) -> u32 {
    it.fuse().fold(0, |sum, n| sum + n)
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
    let names = ["a", "bb", "ccc"];
    println!("{}", fused_sum(names.iter().map(|name| name.len() as u32)));
}
