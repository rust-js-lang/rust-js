// `by_ref()` lends what it's of, never its ownership: std's `&mut I`
// passes on `I`'s `next`, `size_hint`, `nth`, `try_fold` and back steps,
// and its `len` (`core/src/iter/traits/iterator.rs:4233`,
// `double_ended.rs:437`), and its `fold` and `rfold` are `I`'s `try_fold`
// and `try_rfold`, so neither reaches the crate's own `fold` or `rfold`,
// which would take and drop it. Its owner drops it once, at its scope's
// end. In generic code too, where `I` is a type parameter (#745, #747,
// #749).
struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

struct Acc(u32);

impl Drop for Acc {
    fn drop(&mut self) {
        println!("drop acc {}", self.0);
    }
}

struct Total(u32);

impl Drop for Total {
    fn drop(&mut self) {
        println!("drop total {}", self.0);
    }
}

impl std::iter::Sum<D> for Total {
    fn sum<I: Iterator<Item = D>>(iter: I) -> Self {
        iter.fold(Total(0), |mut total, x| {
            total.0 += x.0;
            total
        })
    }
}

// The crate's iterator, which writes its own `fold`, `rfold` and
// `size_hint`, each saying so.
struct Every {
    at: u32,
    end: u32,
}

impl Drop for Every {
    fn drop(&mut self) {
        println!("drop every");
    }
}

impl Iterator for Every {
    type Item = D;
    fn next(&mut self) -> Option<D> {
        (self.at < self.end).then(|| {
            self.at += 1;
            D(self.at)
        })
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        println!("own size_hint");
        let left = (self.end - self.at) as usize;
        (left, Some(left))
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

impl DoubleEndedIterator for Every {
    fn next_back(&mut self) -> Option<D> {
        (self.at < self.end).then(|| {
            self.end -= 1;
            D(self.end + 1)
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

impl ExactSizeIterator for Every {}

fn every(end: u32) -> Every {
    Every { at: 0, end }
}

fn ds() -> Vec<D> {
    vec![D(1), D(2), D(3), D(4), D(5)]
}

fn lent_of<I: DoubleEndedIterator<Item = D> + ExactSizeIterator>(src: I) {
    let mut held = src;
    println!("hint {:?}", held.by_ref().size_hint());
    let total: Total = held.by_ref().take(2).sum();
    println!("sum {}", total.0);
    let acc = held.by_ref().rfold(Acc(0), |mut acc, x| {
        acc.0 += x.0;
        acc
    });
    println!("rfold {}", acc.0);
}

fn mapped<I: DoubleEndedIterator<Item = D> + ExactSizeIterator>(src: I) {
    let mut held = src;
    let firsts: Vec<u32> = held.by_ref().map(|d| d.0 * 10).take(2).collect();
    println!("mapped {firsts:?}, {} left", held.len());
    let all: Vec<u32> = held.by_ref().map(|d| d.0).collect();
    println!("mapped {all:?}");
}

fn summed<I: DoubleEndedIterator<Item = D> + ExactSizeIterator>(src: I) {
    let mut held = src;
    let total: Total = held.by_ref().sum();
    println!("sum {}", total.0);
}

fn back_of<I: DoubleEndedIterator<Item = D>>(it: &mut I) -> Option<u32> {
    it.nth_back(2).map(|d| d.0)
}

fn six() -> Vec<D> {
    vec![D(1), D(2), D(3), D(4), D(5), D(6)]
}

fn main() {
    // A `rev()` of one lent drops nothing of it, as it ends early or as
    // it's dropped; and what it passes over from its back, a `Vec`'s, is
    // dropped as one slice, front to back, as `IntoIter::advance_back_by`
    // drops it.
    let mut lender = six().into_iter();
    lender.by_ref().rev().take(1).for_each(|d| println!("took {}", d.0));
    println!("left {}", lender.len());
    let firsts: Vec<D> = lender.by_ref().rev().take(2).collect();
    println!("took {} left {}", firsts.len(), lender.len());
    let mut found_in = six().into_iter();
    let found = found_in.by_ref().rev().find(|d| d.0 == 4).map(|d| d.0);
    println!("found {found:?} left {}", found_in.len());
    let mut nth_of = six().into_iter();
    println!("rev nth {:?}", nth_of.by_ref().rev().nth(2).map(|d| d.0));
    let mut generic = six().into_iter();
    println!("generic {:?}", back_of(&mut generic.by_ref()));
    let mut skipped = six().into_iter();
    println!("rev skip {:?}", skipped.by_ref().rev().skip(2).next().map(|d| d.0));
    // Its length and hint, of a `Vec`'s and the crate's.
    let mut measured = ds().into_iter();
    println!("len {}", measured.by_ref().len());
    let lent_hint = measured.by_ref().size_hint();
    println!("hint {lent_hint:?}");
    measured.next();
    println!("len {} after one", measured.by_ref().len());
    let mut own_hinted = every(3);
    let own_hint = own_hinted.by_ref().size_hint();
    println!("hint {own_hint:?}");
    // A reversed range held, std's `Rev`: its source's hint.
    let reversed = (1..6u32).rev();
    let range_hint = reversed.size_hint();
    println!("hint {range_hint:?}");
    // Its `rfold` and `fold`, the crate's `next_back` and `next`, never
    // its own `rfold` or `fold`.
    let mut seen = 0;
    let mut folded_back = every(4);
    let from_back = folded_back.by_ref().rfold(Acc(0), |mut acc, x| {
        seen += 1;
        acc.0 += x.0;
        acc
    });
    println!("rfold {} after {seen}", from_back.0);
    let mut folded = every(3);
    let acc = folded.by_ref().fold(Acc(0), |mut acc, x| {
        acc.0 += x.0;
        acc
    });
    println!("fold {}", acc.0);
    // In generic code.
    summed(every(3));
    summed(ds().into_iter());
    lent_of(every(5));
    lent_of(ds().into_iter());
    mapped(ds().into_iter());
}
