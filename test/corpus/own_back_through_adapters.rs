// The crate's own `rfold` and `nth_back`, as std's adapters ask them (#220):
// a `map`'s, a `filter`'s, an `enumerate`'s and a `chain`'s `rfold` are
// their sources'; a `rev()`'s fold is its source's `rfold`, its `nth` its
// `nth_back`, else std's, stepped by `next_back`; an `enumerate`'s and a
// `skip`'s `nth_back` are their source's, a `chain`'s its first side's; a
// `take` steps its source's `nth_back` past what it won't give, from its
// back, by its `rfold`, by its `try_rfold`, and so by a `filter`'s
// `next_back`, which is its source's `rfind`, and a `skip`'s `rfold`.
struct Count {
    at: i32,
    end: i32,
}

impl Iterator for Count {
    type Item = i32;

    fn next(&mut self) -> Option<i32> {
        if self.at < self.end {
            self.at += 1;
            Some(self.at)
        } else {
            None
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let n = (self.end - self.at).max(0) as usize;
        (n, Some(n))
    }
}

impl DoubleEndedIterator for Count {
    fn next_back(&mut self) -> Option<i32> {
        if self.at < self.end {
            self.end -= 1;
            Some(self.end + 1)
        } else {
            None
        }
    }

    fn nth_back(&mut self, n: usize) -> Option<i32> {
        println!("nth_back {n}");
        self.end = self.end.saturating_sub(n as i32);
        self.next_back()
    }

    fn rfold<B, F: FnMut(B, i32) -> B>(mut self, init: B, mut f: F) -> B {
        println!("rfold");
        let mut acc = init;
        while let Some(x) = self.next_back() {
            acc = f(acc, x);
        }
        acc
    }
}

impl ExactSizeIterator for Count {}

fn count() -> Count {
    Count { at: 0, end: 9 }
}

/// One with a destructor, which its own `rfold` drops, once.
struct Loud {
    left: i32,
}

impl Iterator for Loud {
    type Item = i32;

    fn next(&mut self) -> Option<i32> {
        (self.left > 0).then(|| {
            self.left -= 1;
            3 - self.left
        })
    }
}

impl DoubleEndedIterator for Loud {
    fn next_back(&mut self) -> Option<i32> {
        (self.left > 0).then(|| {
            self.left -= 1;
            self.left + 1
        })
    }

    fn rfold<B, F: FnMut(B, i32) -> B>(mut self, init: B, mut f: F) -> B {
        println!("loud rfold");
        let mut acc = init;
        while let Some(x) = self.next_back() {
            acc = f(acc, x);
        }
        acc
    }
}

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop loud {}", self.left);
    }
}

fn main() {
    println!("{}", count().map(|x| x * 2).rev().sum::<i32>());
    println!("{}", count().filter(|x| x % 2 == 1).rfold(0, |a, x| a + x));
    let last = count().map(|x| x * 2).rev().last();
    println!("{last:?}");
    println!("{}", count().chain(count()).rev().count());
    let second = count().chain(count()).rev().nth(10);
    println!("{second:?}");
    let pair = count().enumerate().rev().nth(1);
    let indexed = count().enumerate().rfold(0, |a, (i, x)| a + i as i32 * x);
    println!("{pair:?} {indexed}");
    println!("{}", count().take(5).rfold(0, |a, x| a + x));
    println!("{:?}", count().take(5).rev().collect::<Vec<_>>());
    let fourth = count().take(5).rev().nth(1);
    let skipped = count().skip(2).rev().nth(1);
    println!("{fourth:?} {skipped:?}");
    let trimmed = count().take(5).skip(2).rev().last();
    let odd = count().take(5).filter(|x| x % 2 == 1).next_back();
    let doubled = count().take(5).map(|x| x * 2).rev().nth(1);
    println!("{trimmed:?} {odd:?} {doubled:?}");
    println!("{}", count().take(5).rev().take(2).sum::<i32>());
    let found = count().take(5).rev().find(|x| x % 2 == 0);
    let mut it = count();
    let lent = (&mut it).rev().nth(2);
    println!("{found:?} {lent:?} {:?}", it.next_back());
    println!("{}", Loud { left: 3 }.map(|x| x * 10).rfold(0, |a, x| a + x));
    let skipped_last = count().skip(1).take(5).rev().last();
    let pair = count().enumerate().nth_back(1);
    println!("{skipped_last:?} {pair:?}");
    println!("{}", count().take(5).map(|x| x * 2).rev().take(2).sum::<i32>());
}
