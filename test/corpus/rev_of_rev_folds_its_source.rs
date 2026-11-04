// A `rev()` of a `rev()` is stepped as std's `Rev` steps it: its `last()`
// folds it, never the crate's own `last`, which std's `Rev` doesn't pass
// on, in generic code given it (#621); and a fold of a `rev()` from its
// back is its source's `fold`, as `Rev::rfold` is, an `enumerate()`'s of
// the crate's iterator, which folds by its own (#625). An array's items'
// `rev().rev()` is themselves.
struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

struct Own {
    front: u32,
    back: u32,
}

impl Iterator for Own {
    type Item = D;

    fn next(&mut self) -> Option<D> {
        if self.front == self.back {
            return None;
        }
        self.front += 1;
        Some(D(self.front))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let n = (self.back - self.front) as usize;
        (n, Some(n))
    }

    fn fold<B, F: FnMut(B, D) -> B>(mut self, init: B, mut f: F) -> B {
        println!("own fold");
        let mut acc = init;
        while let Some(d) = self.next() {
            acc = f(acc, d);
        }
        acc
    }

    fn last(mut self) -> Option<D> {
        println!("own last");
        let mut last = None;
        while let Some(d) = self.next() {
            last = Some(d);
        }
        last
    }

    fn count(mut self) -> usize {
        println!("own count");
        let mut n = 0;
        while self.next().is_some() {
            n += 1;
        }
        n
    }
}

impl DoubleEndedIterator for Own {
    fn next_back(&mut self) -> Option<D> {
        if self.front == self.back {
            return None;
        }
        self.back -= 1;
        Some(D(self.back + 1))
    }
}

impl ExactSizeIterator for Own {}

impl Drop for Own {
    fn drop(&mut self) {
        println!("drop own");
    }
}

/// One that writes its own `last` and `count`, not `fold`.
struct Lasts(u32);

impl Iterator for Lasts {
    type Item = D;

    fn next(&mut self) -> Option<D> {
        if self.0 == 3 {
            return None;
        }
        self.0 += 1;
        Some(D(self.0))
    }

    fn last(self) -> Option<D> {
        println!("own last");
        Some(D(3))
    }

    fn count(self) -> usize {
        println!("own count");
        3
    }
}

impl DoubleEndedIterator for Lasts {
    fn next_back(&mut self) -> Option<D> {
        self.next()
    }
}

fn last_of<I: DoubleEndedIterator<Item = D>>(items: I) -> Option<u32> {
    items.rev().rev().last().map(|d| d.0)
}

fn count_of<I: DoubleEndedIterator<Item = D>>(items: I) -> usize {
    items.rev().rev().count()
}

fn own() -> Own {
    Own { front: 0, back: 4 }
}

fn main() {
    println!("last {:?}", own().rev().rev().last().map(|d| d.0));
    println!("count {}", own().rev().rev().count());
    println!("generic last {:?}", last_of(Lasts(0)));
    println!("generic count {}", count_of(Lasts(0)));
    let n = own().enumerate().rev().rfold(0, |n, (i, d)| n * 10 + i as u32 + d.0);
    println!("enumerate rev rfold {n}");
    let v: Vec<D> = (1..4).map(D).collect();
    println!("vec last {:?}", v.into_iter().rev().rev().last().map(|d| d.0));
}
