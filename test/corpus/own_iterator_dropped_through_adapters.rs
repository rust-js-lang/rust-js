// An iterator of the crate's with a destructor, through std's adapters
// (#225): a source closed to drop it is done with however it's stepped to
// its end, by its front, its back, its `nth` or its `nth_back`, and passes
// a compiled `fuse()` its back and length; a `skip` or a `take` ended from
// its back, a `take`'s `try_fold` asked again at its limit, and a `skip`'s
// and a `take`'s `rfold` are done with what they step; a `filter` stepped
// from its back closes its source; what `rev()` and an `enumerate()` made
// knowing its back take is closed to drop it, but not through a `&mut`,
// whose own folds aren't asked, nor its lent JS iterator stepped as it.
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

    fn nth(&mut self, n: usize) -> Option<i32> {
        self.at = self.at.saturating_add(n as i32);
        self.next()
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
        self.end = self.end.saturating_sub(n as i32);
        self.next_back()
    }

    fn rfold<B, F: FnMut(B, i32) -> B>(mut self, init: B, mut f: F) -> B {
        let mut acc = init;
        while let Some(x) = self.next_back() {
            acc = f(acc, x);
        }
        acc
    }
}

impl ExactSizeIterator for Count {}

impl Drop for Count {
    fn drop(&mut self) {
        println!("drop {} {}", self.at, self.end);
    }
}

fn count() -> Count {
    Count { at: 0, end: 9 }
}

struct Loud(u32);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop loud {}", self.0);
    }
}

fn main() {
    println!("{}", count().fuse().map(|x| x * 2).rev().sum::<i32>());
    println!("{}", count().fuse().skip(2).rev().sum::<i32>());
    println!("{}", count().skip(2).rev().sum::<i32>());
    println!("{:?}", count().take(5).rev().collect::<Vec<_>>());
    let odd = count().filter(|x| x % 2 == 1).next_back();
    println!("{odd:?}");
    println!("{:?}", count().map(|x| x * 2).rev().collect::<Vec<_>>());
    println!("{}", count().step_by(2).sum::<i32>());
    let last = count().take(5).filter(|x| x % 2 == 1).last();
    println!("{last:?}");
    println!("{}", count().take(5).skip(2).rev().sum::<i32>());
    let second = count().fuse().rev().nth(1);
    let indexed = count().fuse().enumerate().map(|(i, x)| x + i as i32).nth(1);
    println!("{second:?} {indexed:?}");
    let mut held = count();
    let pair = (&mut held).enumerate().rev().nth(1);
    let doubled = (&mut held).map(|x| x * 2).rev().sum::<i32>();
    let left = (&mut held).enumerate().rev().count();
    println!("{pair:?} {doubled} {left} still held");
    let louds = vec![Loud(1), Loud(2), Loud(3)];
    let second = louds.into_iter().rev().nth(1).map(|l| l.0);
    println!("{second:?} end");
}
